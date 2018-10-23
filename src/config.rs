use std::path::PathBuf;
use std::{u32, u8};

use clap::ArgMatches;
use failure::{Error, ResultExt};

use super::CliError;

/// Data to write.
#[derive(Debug)]
pub enum WriteData {
    /// 32-bit word.
    Word(u32),
    /// Input file.
    File(Box<PathBuf>),
}

/// CLI command.
#[derive(Debug)]
pub enum Command {
    /// U-Boot file.
    Uboot { file: PathBuf, start_uboot: bool },
    /// Dump memory address.
    Dump {
        address: Option<u32>,
        size: Option<u32>,
        hex: bool,
        sid: bool,
        out: Option<PathBuf>,
    },
    /// Write data to memory addresses.
    Write {
        addresses: Vec<u32>,
        data: Vec<WriteData>,
    },
    /// Call function at address.
    Execute { address: u32 },
    /// RMR request for AArch64 warm boot.
    Reset64 { address: u32 },
    /// Get SoC version information.
    Version,
    /// Clear the memory.
    Clear { address: u32, num_bytes: u32 },
    /// Fill the memory.
    Fill {
        address: u32,
        num_bytes: u32,
        fill_byte: u8,
    },
}

/// Configuration structure.
pub struct Config {
    device: Option<(u8, u8)>,
    command: Option<Command>,
}

impl Config {
    /// Generate the config structure from the CLI.
    pub fn from_cli(cli: &ArgMatches) -> Result<Self, Error> {
        Ok(Self {
            device: Self::get_device_from_cli(&cli)?,
            command: Self::get_command_from_cli(&cli)?,
        })
    }

    /// Gets the USB bus and address of the FEL device if provided in the CLI.
    pub fn get_device(&self) -> Option<(u8, u8)> {
        self.device
    }

    /// Gets the command used in the CLI.
    pub fn get_command(&self) -> Option<&Command> {
        self.command.as_ref()
    }

    /// Gets the device information from the CLI.
    fn get_device_from_cli(cli: &ArgMatches) -> Result<Option<(u8, u8)>, Error> {
        Ok(match cli.value_of("device") {
            Some(device_str) => {
                let mut split = device_str.split(':');
                let bus = split.next();
                let addr = split.next();
                if let (Some(bus), Some(addr), None) = (bus, addr, split.next()) {
                    Some((
                        bus.parse::<u8>().context(CliError {
                            description: format!(
                                "bus number must be an integeer between 0 and {}",
                                u8::max_value()
                            ),
                        })?,
                        addr.parse::<u8>().context(CliError {
                            description: format!(
                                "device address must be an integeer between 0 and {}",
                                u8::max_value()
                            ),
                        })?,
                    ))
                } else {
                    return Err(CliError {
                        description: "Device must be in `bus:addr` format".to_owned(),
                    }.into());
                }
            }
            None => None,
        })
    }

    /// Gets the command used in te CLI.
    fn get_command_from_cli(cli: &ArgMatches) -> Result<Option<Command>, Error> {
        if let Some(spl) = cli.subcommand_matches("spl") {
            let file = PathBuf::from(spl.value_of("file").unwrap());
            if file.exists() {
                Ok(Some(Command::Uboot {
                    file,
                    start_uboot: spl.is_present("exec"),
                }))
            } else {
                Err(CliError {
                    description: format!("the file '{}' does not exist", file.display()),
                }.into())
            }
        } else if let Some(dump) = cli.subcommand_matches("dump") {
            if dump.is_present("sid") {
                Ok(Some(Command::Dump {
                    address: None,
                    size: None,
                    hex: false,
                    sid: true,
                    out: None,
                }))
            } else {
                let addr_str = dump.value_of("addr").unwrap();
                let addr = if addr_str.starts_with("0x") {
                    u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(addr_str, 10)
                }.context(CliError {
                    description: format!(
                        "memory address must be an integer from 0x00000000 to {:#010x}",
                        u32::max_value()
                    ),
                })?;
                let size = if let Some(size_str) = dump.value_of("size") {
                    let size = if size_str.starts_with("0x") {
                        u32::from_str_radix(size_str.trim_left_matches("0x"), 16)
                    } else {
                        u32::from_str_radix(size_str, 10)
                    }.context(CliError {
                        description: format!(
                            "dump size must be an integer from 0x00000000 to {:#010x} (the \
                             maximum size starting from the given address)",
                            (u32::max_value() - addr).saturating_add(1)
                        ),
                    })?;
                    if size > (u32::max_value() - addr).saturating_add(1) {
                        return Err(CliError {
                            description: format!(
                                "dump size must be an integer from 0x00000000 to {:#010x} (the \
                                 maximum size starting from the given address)",
                                (u32::max_value() - addr).saturating_add(1)
                            ),
                        }.into());
                    }
                    Some(size)
                } else {
                    None
                };
                Ok(Some(Command::Dump {
                    address: Some(addr),
                    size,
                    hex: dump.is_present("hex"),
                    sid: false,
                    out: dump.value_of("out").map(PathBuf::from),
                }))
            }
        } else if let Some(write) = cli.subcommand_matches("write") {
            let mut value_iter = write.values_of("write_data").unwrap();
            let writes = (write.occurrences_of("write_data") / 2) as usize;
            let mut addresses = Vec::with_capacity(writes);
            let mut data = Vec::with_capacity(writes);
            for _ in 0..writes {
                let addr_str = value_iter.next().unwrap();
                let addr = if addr_str.starts_with("0x") {
                    u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(addr_str, 10)
                }.context(CliError {
                    description: format!(
                        "memory address must be an integer from 0x00000000 to {:#010x}, given \
                         '{}'",
                        u32::max_value(),
                        addr_str
                    ),
                })?;
                let value_str = value_iter.next().unwrap();
                let word = if value_str.starts_with("0x") {
                    u32::from_str_radix(value_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(value_str, 10)
                };
                let final_value = match word {
                    Ok(word) => {
                        if u32::max_value() - 4 >= addr {
                            WriteData::Word(word)
                        } else {
                            let err_msg = format!(
                                "cannot write a complete word at address {:#010x}, it would write \
                                 past the end of the memory address space (limit: {:#010x})",
                                addr,
                                u32::max_value()
                            );
                            return Err(CliError {
                                description: err_msg,
                            }.into());
                        }
                    }
                    Err(e) => {
                        let path = PathBuf::from(value_str);
                        if path.exists() {
                            let metadata =
                                path.metadata().context("could not read file metadata")?;
                            let max_bytes = u64::from((u32::max_value() - addr).saturating_add(1));
                            if metadata.len() > max_bytes {
                                let err_msg = format!(
                                    "the file '{}' is too big. The maximum file size to write to \
                                     address {:#010x} is {} bytes, but the file had {} bytes",
                                    path.display(),
                                    addr,
                                    max_bytes,
                                    metadata.len()
                                );
                                return Err(CliError {
                                    description: err_msg,
                                }.into());
                            }
                            WriteData::File(Box::new(path))
                        } else {
                            return Err(CliError {
                                description: format!(
                                "the file '{}' does not exist.\nNote: If you were trying to \
                                 provide a value, the integeer conversion failed with this error: \
                                 {}",
                                path.display(),
                                e
                            ),
                            }.into());
                        }
                    }
                };
                addresses.push(addr);
                data.push(final_value);
            }
            Ok(Some(Command::Write { addresses, data }))
        } else if let Some(exec) = cli.subcommand_matches("exec") {
            let addr_str = exec.value_of("addr").unwrap();
            let addr = if addr_str.starts_with("0x") {
                u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
            } else {
                u32::from_str_radix(addr_str, 10)
            }.context(CliError {
                description: format!(
                    "memory address must be an integer from 0x00000000 to {:#010x}, given '{}'",
                    u32::max_value(),
                    addr_str
                ),
            })?;
            Ok(Some(Command::Execute { address: addr }))
        } else if let Some(reset64) = cli.subcommand_matches("reset64") {
            let addr_str = reset64.value_of("addr").unwrap();
            let addr = if addr_str.starts_with("0x") {
                u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
            } else {
                u32::from_str_radix(addr_str, 10)
            }.context(CliError {
                description: format!(
                    "memory address must be an integer from 0x00000000 to {:#010x}, given '{}'",
                    u32::max_value(),
                    addr_str
                ),
            })?;
            Ok(Some(Command::Reset64 { address: addr }))
        } else if cli.subcommand_matches("version").is_some() {
            Ok(Some(Command::Version))
        } else if let Some(clear) = cli.subcommand_matches("clear") {
            let addr_str = clear.value_of("addr").unwrap();
            let address = if addr_str.starts_with("0x") {
                u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
            } else {
                u32::from_str_radix(addr_str, 10)
            }.context(CliError {
                description: format!(
                    "memory address must be an integer from 0x00000000 to {:#010x}, given '{}'",
                    u32::max_value(),
                    addr_str
                ),
            })?;
            let num_bytes_str = clear.value_of("num_bytes").unwrap();
            let num_bytes = if num_bytes_str.starts_with("0x") {
                u32::from_str_radix(num_bytes_str.trim_left_matches("0x"), 16)
            } else {
                u32::from_str_radix(num_bytes_str, 10)
            }.context(CliError {
                description: format!(
                    "the number of bytes to clear must be an integer from 0x00000000 to {:#010x} \
                     (the maximum size starting from the given address)",
                    (u32::max_value() - address).saturating_add(1)
                ),
            })?;
            if num_bytes > (u32::max_value() - address).saturating_add(1) {
                return Err(CliError {
                    description: format!(
                    "clear size must be an integer from 0x00000000 to {:#010x} (the maximum size \
                     starting from the given address)",
                    (u32::max_value() - address).saturating_add(1)
                ),
                }.into());
            }

            Ok(Some(Command::Clear { address, num_bytes }))
        } else if let Some(fill) = cli.subcommand_matches("fill") {
            let addr_str = fill.value_of("addr").unwrap();
            let address = if addr_str.starts_with("0x") {
                u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
            } else {
                u32::from_str_radix(addr_str, 10)
            }.context(CliError {
                description: format!(
                    "memory address must be an integer from 0x00000000 to {:#010x}, given '{}'",
                    u32::max_value(),
                    addr_str
                ),
            })?;
            let num_bytes_str = fill.value_of("num_bytes").unwrap();
            let num_bytes = if num_bytes_str.starts_with("0x") {
                u32::from_str_radix(num_bytes_str.trim_left_matches("0x"), 16)
            } else {
                u32::from_str_radix(num_bytes_str, 10)
            }.context(CliError {
                description: format!(
                    "the number of bytes to fill must be an integer from 0x00000000 to {:#010x} \
                     (the maximum size starting from the given address)",
                    (u32::max_value() - address).saturating_add(1)
                ),
            })?;
            let fill_byte_str = fill.value_of("fill_byte").unwrap();
            let fill_byte = if fill_byte_str.starts_with("0x") {
                u8::from_str_radix(fill_byte_str.trim_left_matches("0x"), 16)
            } else {
                u8::from_str_radix(fill_byte_str, 10)
            }.context(CliError {
                description: format!(
                    "the filling byte must be an integer from 0x00 to {:#04x}, given '{}'",
                    u8::max_value(),
                    fill_byte_str
                ),
            })?;
            Ok(Some(Command::Fill {
                address,
                num_bytes,
                fill_byte,
            }))
        } else {
            Ok(None)
        }
    }
}
