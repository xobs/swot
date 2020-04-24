use ftdi_vcp_rs::{Error, VCP};
use std::thread::sleep;
use std::time::Duration;

pub enum EraseType {
    Kb64,
}

pub struct Flash {
    pub vcp: VCP,
    verbose: bool,
}

impl Flash {
    pub fn new(vcp: VCP) -> Flash {
        Flash { vcp, verbose: false }
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    fn set_cs_creset(&mut self, cs_asserted: bool, reset_asserted: bool) -> Result<(), Error> {
        let gpio = if cs_asserted { 0x10 } else { 0 } | if reset_asserted { 0x80 } else { 0 };
        let direction = 0x93;
        self.vcp.set_gpio(gpio, direction)
    }

    // the FPGA reset is released so also FLASH chip select should be deasserted
    pub fn release_reset(&mut self) -> Result<(), Error> {
        self.set_cs_creset(true, true)
    }

    // FLASH chip select assert
    // should only happen while FPGA reset is asserted
    pub fn chip_select(&mut self) -> Result<(), Error> {
        self.set_cs_creset(false, false)
    }

    // FLASH chip select deassert
    pub fn chip_deselect(&mut self) -> Result<(), Error> {
        self.set_cs_creset(true, false)
    }

    pub fn reset(&mut self) -> Result<(), ftdi_vcp_rs::Error> {
        self.chip_select()?;
        self.vcp.xfer_spi_bits(0xFF, 8)?;
        self.chip_deselect()?;

        self.chip_select()?;
        self.vcp.xfer_spi_bits(0xFF, 2)?;
        self.chip_deselect()?;
        Ok(())
    }

    pub fn power_up(&mut self) -> Result<(), Error> {
        let mut cmd = [0xAB /* FC_RPD */];
        self.chip_select()?;
        self.vcp.xfer_spi(&mut cmd)?;
        self.chip_deselect()?;
        Ok(())
    }

    pub fn power_down(&mut self) -> Result<(), Error> {
        let mut cmd = [0xB9 /* FC_PD */];
        self.chip_select()?;
        self.vcp.xfer_spi(&mut cmd)?;
        self.chip_deselect()?;
        Ok(())
    }

    pub fn read_id(&mut self) -> Result<(), Error> {
        /* JEDEC ID structure:
         * Byte No. | Data Type
         * ---------+----------
         *        0 | FC_JEDECID Request Command
         *        1 | MFG ID
         *        2 | Dev ID 1
         *        3 | Dev ID 2
         *        4 | Ext Dev Str Len
         */
        let mut data = [0xffu8; 260];
        data[0] = 0x9Fu8 /* FC_JEDECID - Read JEDEC ID */;
        let mut len = 5usize; // command + 4 response bytes

        if self.verbose {
            println!("read flash ID..");
        }

        self.chip_select()?;

        // Write command and read first 4 bytes
        self.vcp.xfer_spi(&mut data[0..=4])?;

        if data[4] == 0xFF {
            println!(
                "Extended Device String Length is 0xFF, this is likely a read error. Ignorig..."
            );
        } else {
            // Read extended JEDEC ID bytes
            if data[4] != 0 {
                len += data[4] as usize;
                let (_, mut jedec_data) = data.split_at_mut(4);
                self.vcp.xfer_spi(&mut jedec_data)?;
            }
        }

        self.chip_deselect()?;

        // TODO: Add full decode of the JEDEC ID.
        print!("flash ID:");
        for b in &data[1..len] {
            print!(" 0x{:02X}", b);
        }
        println!();
        Ok(())
    }

    pub fn cdone(&mut self) -> Result<bool, Error> {
        // ADBUS6 (GPIOL2)
        match self.vcp.readb_low()? & 0x40 {
            0 => Ok(false),
            _ => Ok(true),
        }
    }

    pub fn cdone_str(&mut self) -> Result<&'static str, Error> {
        if self.cdone()? {
            Ok("high")
        } else {
            Ok("low")
        }
    }

    pub fn write_enable(&mut self) -> Result<(), Error> {
        if self.verbose {
            println!("status before enable:");
            self.read_status()?;
            println!("write enable..");
        }

        let mut data = [0x06 /* FC_WE // Write Enable */];
        self.chip_select()?;
        self.vcp.xfer_spi(&mut data)?;
        self.chip_deselect()?;

        if self.verbose {
            println!("status after enable:");
            self.read_status()?;
        }
        Ok(())
    }

    pub fn bulk_erase(&mut self) -> Result<(), Error> {
        println!("bulk erase..");
        let mut data = [0xC7 /* FC_CE // Chip Erase */];
        self.chip_select()?;
        self.vcp.xfer_spi(&mut data)?;
        self.chip_deselect()?;
        Ok(())
    }

    pub fn sector_erase(&mut self, erase_type: EraseType, addr: usize) -> Result<(), Error> {
        let erase_cmd = match erase_type {
            EraseType::Kb64 => 0xD8, /* FC_BE64 // Block Erase 64kb */
        };

        println!("erase 64kB sector at 0x{:06X}..", addr);
        self.chip_select()?;
        self.vcp
            .send_spi(&[erase_cmd, (addr >> 16) as u8, (addr >> 8) as u8, addr as u8])?;
        self.chip_deselect()?;
        Ok(())
    }

    pub fn read_status(&mut self) -> Result<u8, Error> {
        let mut data = [0x05 /* FC_RSR1 // Read Status Register 1 */, 0x00];

        self.chip_select()?;
        self.vcp.xfer_spi(&mut data)?;
        self.chip_deselect()?;

        if self.verbose {
            println!("SR1: 0x{:02X}", data[1]);
            println!(
                " - SPRL: {}",
                if data[1] & (1 << 7) == 0 {
                    "unlocked"
                } else {
                    "locked"
                }
            );
            println!(
                " -  SPM: {}",
                if data[1] & (1 << 6) == 0 {
                    "Byte/Page Prog Mode"
                } else {
                    "Sequential Prog Mode"
                }
            );
            println!(
                " -  EPE: {}\n",
                if data[1] & (1 << 5) == 0 {
                    "Erase/Prog success"
                } else {
                    "Erase/Prog error"
                }
            );
            println!(
                "-  SPM: {}\n",
                if data[1] & (1 << 4) == 0 {
                    "~WP asserted"
                } else {
                    "~WP deasserted"
                }
            );
            println!(
                " -  SWP: {}",
                match (data[1] >> 2) & 0x3 {
                    0 => "All sectors unprotected",
                    1 => "Some sectors protected",
                    2 => "Reserved (xxxx 10xx)",
                    3 => "All sectors protected",
                    _ => panic!("math is broken!"),
                }
            );
            println!(
                " -  WEL: {}",
                if (data[1] & (1 << 1)) == 0 {
                    "Not write enabled"
                } else {
                    "Write enabled"
                }
            );
            println!(
                " - ~RDY: {}",
                if (data[1] & (1 << 0)) == 0 {
                    "Ready"
                } else {
                    "Busy"
                }
            );
        }

        sleep(Duration::from_micros(1_000));

        Ok(data[1])
    }

    pub fn disable_protection(&mut self) -> Result<(), Error> {
        println!("disable flash protection...");

        // Write Status Register 1 <- 0x00
        let mut data = [0x01 /* FC_WSR1 // Write Status Register 1 */, 0x00];
        self.chip_select()?;
        self.vcp.xfer_spi(&mut data)?;
        self.chip_deselect()?;

        self.wait()?;

        // Read Status Register 1
        data[0] = 0x05; // FC_RSR1;

        self.chip_select()?;
        self.vcp.xfer_spi(&mut data)?;
        self.chip_deselect()?;

        if data[1] != 0x00 {
            println!(
                "failed to disable protection, SR now equal to 0x{:02x} (expected 0x00)\n",
                data[1]
            );
        }

        Ok(())
    }

    pub fn wait(&mut self) -> Result<(), Error> {
        if self.verbose {
            println!("waiting..");
        }

        let mut count = 0;
        loop {
            let mut data = [0x05 /* FC_RSR1 // Read Status Register 1 */, 0x00];

            self.chip_select()?;
            self.vcp.xfer_spi(&mut data)?;
            self.chip_deselect()?;

            if (data[1] & 0x01) == 0 {
                if count < 2 {
                    count += 1;
                    if self.verbose {
                        print!("r");
                        //fflush(stderr);
                    }
                } else {
                    if self.verbose {
                        print!("R");
                        // fflush(stderr);
                    }
                    break;
                }
            } else {
                if self.verbose {
                    print!(".");
                    // fflush(stderr);
                }
                count = 0;
            }

            sleep(Duration::from_micros(1_000));
        }

        if self.verbose {
            println!();
        }

        Ok(())
    }

    pub fn prog(&mut self, addr: usize, data: &[u8]) -> Result<(), Error> {
        if self.verbose {
            println!("prog 0x{:06X} +0x{:03X}..", addr, data.len());
        }

        let command = [
            0x02, /* FC_PP // Page Program */
            (addr >> 16) as u8,
            (addr >> 8) as u8,
            addr as u8,
        ];

        self.chip_select()?;
        self.vcp.send_spi(&command)?;
        self.vcp.send_spi(data)?;
        self.chip_deselect()?;

        // if verbose {
        //     for (int i = 0; i < n; i++)
        //         fprintf(stderr, "%02x%c", data[i], i == n - 1 || i % 32 == 31 ? '\n' : ' ');
        // }
        Ok(())
    }

    pub fn read(&mut self, addr: usize, data: &mut [u8]) -> Result<(), Error> {
        if self.verbose {
            println!("read 0x{:06X} +0x{:03X}..", addr, data.len());
        }

        let command = [
            0x03, /*FC_RD // Read Data */
            (addr >> 16) as u8,
            (addr >> 8) as u8,
            addr as u8,
        ];

        self.chip_select()?;
        self.vcp.send_spi(&command)?;
        self.vcp.xfer_spi(data)?;
        self.chip_deselect()?;

        // if (verbose)
        //     for (int i = 0; i < n; i++)
        //         fprintf(stderr, "%02x%c", data[i], i == n - 1 || i % 32 == 31 ? '\n' : ' ');
        Ok(())
    }
}
