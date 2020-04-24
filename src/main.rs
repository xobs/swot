/*
#define FT_LIST_NUMBER_ONLY			0x80000000
#define FT_LIST_BY_INDEX			0x40000000
#define FT_LIST_ALL					0x20000000

#define FT_LIST_MASK (FT_LIST_NUMBER_ONLY|FT_LIST_BY_INDEX|FT_LIST_ALL)
*/
use clap::{App, Arg};
use ftdi_vcp_rs::{mpsse::Command::*, BitMode, VCP};
use std::fs::File;
use std::io::Seek;
use std::io::{Read, Write};
use std::thread::sleep;
use std::time::Duration;

mod flash;

fn parse_size(input: &str) -> Result<usize, &'static str> {
    let multiple_index = input
        .chars()
        .position(|c| !(c.is_numeric() || c == '.'))
        .unwrap_or(input.len());

    let (value, multiple) = &input.split_at(multiple_index);
    let value = value.parse::<usize>().map_err(|_| "unable to parse")?;
    let multiple = match multiple.trim().to_lowercase().as_str() {
        "m" | "mib" => 1024 * 1024,
        "k" | "kib" => 1024,
        "b" | "" | "bytes" => 1,
        "g" | "gib" => 1024 * 1024 * 1024,
        x => {
            println!("Unrecognized suffix {}", x);
            return Err("unrecognized suffix")
        },
    };
    Ok(value * multiple)
}

#[test]
fn parse_size_sanity() {
    assert_eq!(parse_size("0").unwrap(), 0);
    assert_eq!(parse_size("1024").unwrap(), 1024);
    assert_eq!(parse_size("1k").unwrap(), 1024);
    assert_eq!(parse_size("1K").unwrap(), 1024);
    assert_eq!(parse_size("1 K").unwrap(), 1024);
    assert_eq!(parse_size("1 k").unwrap(), 1024);
    assert_eq!(parse_size("1 kiB").unwrap(), 1024);
    assert_eq!(parse_size("1 M").unwrap(), 1024*1024);
    assert_eq!(parse_size("2 M").unwrap(), 1024*1024*2);
}

fn main() -> Result<(), ftdi_vcp_rs::Error> {
    let mut vcp = VCP::new_from_name("iCEBreaker V1.0e A").expect("couldn't open vcp");
    let slow_clock = false;
    let disable_verify = false;
    let disable_protect = false;

    let mut bitstream = vec![];

    let matches = App::new("SWOT: the Spi Write Out Tool")
        .version(crate_version!())
        .author("Sean Cross <sean@xobs.io>")
        .about("Read and write SPI devices using an FTDI cable")
        .arg(
            Arg::with_name("FILENAME")
                .help("Sets the bitstream file to read or write")
                .required_unless("verbose")
                .required_unless("test")
                .index(1),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("bulk_erase")
                .short("b")
                .help("bulk erase entire flash before writing"),
        )
        .arg(
            Arg::with_name("check_mode")
                .short("c")
                .help("do not write flash, only verify (`check')"),
        )
        .arg(
            Arg::with_name("no_erase")
                .short("n")
                .help("do not erase flash before writing"),
        )
        .arg(
            Arg::with_name("erase_size")
                .short("e")
                .takes_value(true)
                .help("erase flash as if we were writing that number of bytes"),
        )
        .arg(
            Arg::with_name("read_256")
                .short("r")
                .help("read first 256 kB from flash and write to file"),
        )
        .arg(
            Arg::with_name("read_file")
                .short("R")
                .takes_value(true)
                .conflicts_with("read_256")
                .help("read the specified number of bytes from flash"),
        )
        .arg(
            Arg::with_name("offset")
                .short("o")
                .takes_value(true)
                .default_value("0")
                .help("start address for read/write"),
        )
        .arg(Arg::with_name("test").short("t").help("Run tests"))
        .get_matches();

    let verbose = matches.is_present("verbose");
    let test_mode = matches.is_present("test");
    let bulk_erase = matches.is_present("bulk_erase");
    let check_mode = matches.is_present("check_mode");
    let dont_erase = matches.is_present("no_erase");
    let rw_offset = matches.value_of("offset").map_or(0, |e: &str| {
        parse_size(e).expect("unable to parse size")
    });
    let erase_size = matches.value_of("erase_size").map(|e| {
        Some(parse_size(e).unwrap())
    });
    let read_size = if matches.is_present("read_256") {
        Some(256 * 1024)
    } else if let Some(size) = matches.value_of("read_file") {
        Some(parse_size(size).unwrap())
    } else {
        None
    };

    let mut bitstream_file = if let Some(filename) = matches.value_of("FILENAME") {
        if read_size.is_none() && !test_mode {
            let mut file = File::open(filename).expect("Couldn't open path");
            file.read_to_end(&mut bitstream)
                .expect("couldn't read to end");
            Some(file)
        } else {
            Some(File::create(filename).expect("Couldn't create output file"))
        }
    } else {
        None
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
    flash.set_verbose(verbose);
    flash.release_reset()?;

    sleep(Duration::from_micros(100_000));

    if test_mode {
        println!("reset..");

        flash.chip_deselect()?;
        sleep(Duration::from_micros(250_000));

        println!("cdone: {}", flash.cdone_str()?);

        flash.reset()?;
        flash.power_up()?;

        flash.read_id()?;

        flash.power_down()?;

        flash.release_reset()?;
        sleep(Duration::from_micros(250_000));

        println!("cdone: {}", flash.cdone_str()?);
    } else {
        // ---------------------------------------------------------
        // Reset
        // ---------------------------------------------------------
        if verbose {
            println!("reset..");
        }

        flash.chip_deselect()?;
        sleep(Duration::from_micros(250_000));

        if verbose {
            println!("cdone: {}", flash.cdone_str()?);
        }

        flash.reset()?;
        flash.power_up()?;

        flash.read_id()?;

        // ---------------------------------------------------------
        // Program
        // ---------------------------------------------------------

        if read_size.is_none() && !check_mode {
            if disable_protect {
                flash.write_enable()?;
                flash.disable_protection()?;
            }

            if !dont_erase {
                if bulk_erase {
                    flash.write_enable()?;
                    flash.bulk_erase()?;
                    flash.wait()?;
                } else {
                    println!("file size: {}", bitstream.len());

                    let begin_addr = rw_offset & !0xffff;
                    let end_addr = (rw_offset + bitstream.len() + 0xffff) & !0xffff;

                    for addr in (begin_addr..end_addr).step_by(0x10000) {
                        flash.write_enable()?;
                        flash.sector_erase(flash::EraseType::Kb64, addr)?;
                        if verbose {
                            println!("Status after block erase:");
                            flash.read_status()?;
                        }
                        flash.wait()?;
                    }
                }
            }

            if erase_size.is_none() {
                println!("programming..");

                for (idx, page) in bitstream.chunks(256).enumerate() {
                    flash.write_enable()?;
                    flash.prog(rw_offset + idx * 256, page)?;
                    flash.wait()?;
                }
                /* seek to the beginning for second pass */
                bitstream_file.as_mut()
                    .expect("no bitstream file was specified")
                    .seek(std::io::SeekFrom::Start(0))
                    .expect("Couldn't rewind file");
            }
        }

        // ---------------------------------------------------------
        // Read/Verify
        // ---------------------------------------------------------

        if let Some(read_size) = read_size {
            println!("reading {} bytes..", read_size);
            // uint8_t buffer[256];
            let out_file = bitstream_file.as_mut().expect("no output file found");
            let mut buffer = [0u8; 256];
            for addr in (0..read_size).step_by(256) {
                flash.read(rw_offset + addr, &mut buffer)?;
                out_file
                    .write_all(&buffer)
                    .expect("couldn't write to bitstream file");
            }
        } else if erase_size.is_none() && !disable_verify {
            println!("reading..");
            for (idx, page) in bitstream.chunks(256).enumerate() {
                let mut buffer_flash = [0; 256];
                flash.read(rw_offset + idx * 256, &mut buffer_flash)?;
                if !page.iter().zip(buffer_flash.iter()).all(|(a, b)| a == b) {
                    println!("Found difference between flash and file!");
                }
            }

            println!("VERIFY OK");
        }

        // ---------------------------------------------------------
        // Reset
        // ---------------------------------------------------------

        flash.power_down()?;

        flash.release_reset()?;
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
