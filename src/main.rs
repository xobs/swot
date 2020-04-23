/*
#define FT_LIST_NUMBER_ONLY			0x80000000
#define FT_LIST_BY_INDEX			0x40000000
#define FT_LIST_ALL					0x20000000

#define FT_LIST_MASK (FT_LIST_NUMBER_ONLY|FT_LIST_BY_INDEX|FT_LIST_ALL)
*/
use clap::{App, Arg, SubCommand};
use ftdi_vcp_rs::{mpsse::Command::*, BitMode, VCP};
use std::io::{Write, Read};
use std::thread::sleep;
use std::time::Duration;
use std::fs::File;

mod flash;

fn main() -> Result<(), ftdi_vcp_rs::Error> {
    let mut vcp = VCP::new_from_name("iCEBreaker V1.0e A").expect("couldn't open vcp");
    let slow_clock = false;
    let read_mode = false;
    let check_mode = false;
    let dont_erase = false;
    let bulk_erase = false;
    let disable_verify = false;
    let erase_mode = false;
    let rw_offset = 0;
    let disable_protect = false;
    let read_size = 256 * 1024;

    let mut bitstream = vec![];

    let matches = App::new("iCE40 Programmer")
        .version("1.0")
        .author("Sean Cross <sean@xobs.io>")
        .about("Port of Iceprog")
        .arg(
            Arg::with_name("FILENAME")
                .help("Sets the bitstream file to read or write")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("test")
                .short("t")
                .help("Run tests"),
        )
        .get_matches();

    let verbose = matches.is_present("verbose");
    let test_mode = matches.is_present("test");

    let bitstream = {
        let filename = matches.value_of("FILENAME").unwrap();
        let mut file = File::open(filename).expect("Couldn't open path");
        file.read_to_end(&mut bitstream).expect("couldn't read to end");
        bitstream
    };

    println!("Opened VCP: {:?}", vcp);
    vcp.reset()?;
    vcp.purge()?;

    let previous_latency = vcp.latency_timer()?;
    vcp.set_latency_timer(1)?;

    vcp.set_bitmode(0xff, BitMode::MPSSE)?;

    // enable clock divide by 5
    vcp.write(&[MC_TCK_D5.to_u8()])
        .or_else(|_| Err(ftdi_vcp_rs::Error::IoError))?;

    if slow_clock {
        // set 50 kHz clock
        vcp.write(&[MC_SET_CLK_DIV.to_u8(), 119, 0x00])
            .or_else(|_| Err(ftdi_vcp_rs::Error::IoError))?;
    } else {
        // set 6 MHz clock
        vcp.write(&[MC_SET_CLK_DIV.to_u8(), 0x00, 0x00])
            .or_else(|_| Err(ftdi_vcp_rs::Error::IoError))?;
    }

    let mut flash = flash::Flash::new(vcp);
    flash.release_reset()?;

    sleep(Duration::from_micros(100_000));

    if test_mode {
        println!("reset..");

        flash.chip_deselect()?;
        sleep(Duration::from_micros(250_000));

        println!("cdone: {}", flash.cdone_str()?);

        flash.reset()?;
        flash.power_up()?;

        flash.read_id(true)?;

        flash.power_down()?;

        flash.release_reset()?;
        sleep(Duration::from_micros(250_000));

        println!("cdone: {}", flash.cdone_str()?);
    } else {
        // ---------------------------------------------------------
        // Reset
        // ---------------------------------------------------------

        println!("reset..");

        flash.chip_deselect()?;
        sleep(Duration::from_micros(250_000));

        println!("cdone: {}", flash.cdone_str()?);

        flash.reset()?;
        flash.power_up()?;

        flash.read_id(true)?;

        // ---------------------------------------------------------
        // Program
        // ---------------------------------------------------------

        if !read_mode && !check_mode {
            if disable_protect {
                flash.write_enable(verbose)?;
                flash.disable_protection()?;
            }

            if !dont_erase {
                if bulk_erase {
                    flash.write_enable(verbose)?;
                    flash.bulk_erase()?;
                    flash.wait(verbose)?;
                } else {
                    println!("file size: {}", bitstream.len());

                    let begin_addr = rw_offset & !0xffff;
                    let end_addr = (rw_offset + bitstream.len() + 0xffff) & !0xffff;

                    for addr in (begin_addr..end_addr).step_by(0x10000) {
                        flash.write_enable(verbose)?;
                        flash.sector_erase(flash::EraseType::Kb64, addr)?;
                        if verbose {
                            println!("Status after block erase:");
                            flash.read_status(verbose)?;
                        }
                        flash.wait(verbose)?;
                    }
                }
            }

            if !erase_mode {
                println!("programming..");

                for (idx, page) in bitstream.chunks(256).enumerate() {
                	flash.write_enable(verbose)?;
                	flash.prog(rw_offset + idx*256, page, verbose)?;
                	flash.wait(verbose)?;
                }

                /* seek to the beginning for second pass */
                // fseek(f, 0, SEEK_SET);
            }
        }

        // ---------------------------------------------------------
        // Read/Verify
        // ---------------------------------------------------------

        if read_mode {
            println!("reading..");
            for addr in (0..read_size).step_by(256) {
                // uint8_t buffer[256];
                // flash_read(rw_offset + addr, buffer, 256);
                // fwrite(buffer, read_size - addr > 256 ? 256 : read_size - addr, 1, f);
            }
        } else if !erase_mode && !disable_verify {
            println!("reading..");
            for (idx, page) in bitstream.chunks(256).enumerate() {
                let mut buffer_flash = [0; 256];
                flash.read(rw_offset + idx*256, &mut buffer_flash, verbose);
                if ! page.iter().zip(buffer_flash.iter()).all(|(a,b)| a == b) {
                    println!("Found difference between flash and file!");
                }
            }

            println!("VERIFY OK");
        }

        // ---------------------------------------------------------
        // Reset
        // ---------------------------------------------------------

        flash.power_down()?;

        flash.release_reset();
        sleep(Duration::from_micros(250_000));

        println!("cdone: {}", flash.cdone_str()?);
    }

    if let Ok(com_port) = flash.vcp.com_port() {
        println!("VCP COM{}:", com_port);
    } else {
        println!("No COM port assigned");
    }

    flash.vcp.set_latency_timer(previous_latency)?;
    Ok(())
}
