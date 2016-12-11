//! C.H.I.P. flasher.

// #![forbid(missing_docs, warnings)]
#![deny(deprecated, improper_ctypes, non_shorthand_field_patterns, overflowing_literals,
    plugin_as_library, private_no_mangle_fns, private_no_mangle_statics, stable_features,
    unconditional_recursion, unknown_lints, unused, unused_allocation, unused_attributes,
    unused_comparisons, unused_features, unused_parens, while_true)]
#![warn(missing_docs, trivial_casts, trivial_numeric_casts, unused, unused_extern_crates,
    unused_import_braces, unused_qualifications, unused_results, variant_size_differences)]

// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate aw_fel;
#[macro_use]
extern crate clap;
extern crate ansi_term;

use std::io::{self, Write, Read, BufWriter, BufReader};
use std::fs::File;

use aw_fel::{Fel, SPL_LEN_LIMIT};
use ansi_term::Colour::Red;
use ansi_term::Style;

mod error {
    error_chain!{
        errors {
            /// CLI error
            CLI(description: String) {
                description("CLI error")
                display("CLI error: {}", description)
            }
        }
        foreign_links {
            ParseInt(::std::num::ParseIntError);
            Io(::std::io::Error);
        }
        links {
            Fel(::aw_fel::error::Error, ::aw_fel::error::ErrorKind);
        }
    }
}
mod cli;
mod config;

use error::*;
use config::{Config, Command, WriteData};

const HEX_DUMP_LINE: usize = 0x10;

fn main() {
    if let Err(e) = run() {
        io::stderr()
            .write_all(format!("{} {}\n", Red.bold().paint("error:"), e).as_bytes())
            .unwrap();
        for e in e.iter().skip(1) {
            io::stderr()
                .write_all(format!("  {} {}\n", Style::new().bold().paint("caused_by:"), e)
                    .as_bytes())
                .unwrap();
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = Config::from_cli(cli::generate_cli().get_matches())?;
    if config.get_command().is_none() {
        println!("{} No command specified.",
                 Style::new().bold().paint("Warning:"));
        return Ok(());
    }
    let fel = Fel::new().chain_err(|| "unable to initialize the tool")?;

    let device = if let Some((bus, addr)) = config.get_device() {
        if let Some(device) = fel.get_device(bus, addr)? {
            device
        } else {
            bail!("no FEL device found in bus {} with address {}", bus, addr);
        }
    } else {
        let mut dev_list = fel.list_devices()?;
        if dev_list.is_empty() {
            bail!("No FEL devices found.");
        } else {
            dev_list.swap_remove(0)
        }
    };

    match *config.get_command().unwrap() {
        Command::Uboot { ref file, start_uboot } => {
            // Load file.
            let mut reader =
                BufReader::new(File::open(file).chain_err(|| "could not open U-Boot file")?);
            let mut contents = Vec::new();
            let _ = reader.read_to_end(&mut contents)
                .chain_err(|| "could not read U-Boot file")?;

            if start_uboot && contents.len() <= SPL_LEN_LIMIT as usize {
                bail!("the provided file does not contain a valid U-Boot image to be executed");
            }

            // Write and execute the SPL from the buffer.
            device.write_and_execute_spl(&contents)
                .chain_err(|| "there was an error trying to write SPL to memory or executing it")?;

            if contents.len() > SPL_LEN_LIMIT as usize {
                let (entry_point, _) = device.write_uboot_image(&contents[SPL_LEN_LIMIT as usize..])
                    .chain_err(|| "could not write U-Boot image to device after writting the SPL")?;
                if start_uboot {
                    device.fel_execute(entry_point).chain_err(|| "could not execute U-Boot")?;
                } else {
                    println!("{:#010x}", entry_point);
                }
            }
        }
        Command::Dump { address, size, hex, sid, ref out } => {
            if sid {
                if let Some(sid) = device.read_sid()
                    .chain_err(|| "unable to get SID from device")? {
                    println!("{:08x}:{:08x}:{:08x}:{:08x}",
                             sid[0],
                             sid[1],
                             sid[2],
                             sid[3]);
                } else {
                    bail!("the device does not have SID registers");
                }
            } else if size.is_some() {
                let (address, size) = (address.unwrap(), size.unwrap());
                let mut result = vec![0u8; size as usize];
                device.fel_read(address, &mut result)
                    .chain_err(|| {
                        format!("could not read {:#010x} bytes at memory address {:#010x}",
                                size,
                                address)
                    })?;
                if hex {
                    hex_dump(&result, address);
                } else if let Some(ref out_path) = *out {
                    let mut file = BufWriter::new(File::create(out_path)
                                    .chain_err(|| "unable to create output file")?);
                    file.write_all(&result)
                        .chain_err(|| "unable to write dumped data to file")?;
                } else {
                    io::stdout().write_all(&result)
                        .chain_err(|| "unable to write dumped data to stdout")?;
                }
            } else {
                let addr = address.unwrap();
                let mut val = [0u32];
                device.read_words(addr, &mut val)
                    .chain_err(|| format!("unable to read {:#010x} address", addr))?;
                println!("{:#010x}", val[0]);
            }
        }
        Command::Write { ref addresses, ref data } => {
            for (addr, data) in addresses.iter().zip(data) {
                match *data {
                    WriteData::Word(w) => {
                        device.write_words(*addr, &[w])
                            .chain_err(|| {
                                format!("could not write word {:#010x} to address {:#010x}",
                                        w,
                                        addr)
                            })?;
                        println!("Wrote word {:#010x} to address {:#010x}", w, addr);
                    }
                    WriteData::File(ref path) => {
                        let file =
                            File::open(path.as_ref()).chain_err(|| {
                                    format!("could not open the file '{}'", path.display())
                                })?;
                        let mut reader = BufReader::new(file);
                        let mut data = Vec::new();
                        let _ = reader.read_to_end(&mut data)
                            .chain_err(|| {
                                format!("could not read data from file '{}'", path.display())
                            })?;
                        device.fel_write(*addr, &data)
                            .chain_err(|| "could not write file data to device memory")?;

                        println!("Wrote contents of file '{}' to address {:#010x}",
                                 path.display(),
                                 addr);
                    }
                }
            }
        }
        Command::Execute { address } => {
            device.fel_execute(address)
                .chain_err(|| format!("unable to execute code at address {:#010x}", address))?;
        }
        Command::Reset64 { address } => {
            device.rmr_request(address, true)
                .chain_err(|| "could not send the warm RMR reset request")?;
            println!("Warm RMR reset request sent");
        }
        Command::Version => println!("{:?}", device.get_version_info()),
        Command::Clear { address, num_bytes } => {
            device.fel_fill(address, num_bytes, 0x00)
                .chain_err(|| {
                    format!("unable to clear {} bytes at address {:#010x}",
                            num_bytes,
                            address)
                })?;
            println!("Cleared {} bytes at address {:#010x}", num_bytes, address);
        }
        Command::Fill { address, num_bytes, fill_byte } => {
            device.fel_fill(address, num_bytes, fill_byte)
                .chain_err(|| {
                    format!("unable to fill {} bytes at address {:#010x} with byte {:#04x}",
                            num_bytes,
                            address,
                            fill_byte)
                })?;
            println!("Filled {} bytes at address {:#010x} with byte {:#04x}",
                     num_bytes,
                     address,
                     fill_byte);
        }
    }

    Ok(())
}

/// Pretty prints the given hexadecimal dump.
fn hex_dump(data: &[u8], offset: u32) {
    for (i, chunk) in data.chunks(HEX_DUMP_LINE).enumerate() {
        let start_address = offset + (i * HEX_DUMP_LINE) as u32;
        let extra = HEX_DUMP_LINE - chunk.len();
        let mut bytes = String::with_capacity(HEX_DUMP_LINE * 3);
        let mut ascii = String::with_capacity(HEX_DUMP_LINE);
        for byte in chunk {
            let byte = *byte;
            bytes.push_str(&format!("{:02x} ", byte));
            ascii.push(if byte >= 0x20 && byte <= 0x7E {
                char::from(byte)
            } else {
                '.'
            })
        }
        for _ in 0..extra {
            bytes.push_str("__ ");
            ascii.push('.');
        }
        println!("{:08x}: {} {}", start_address, bytes, ascii);
    }
}
