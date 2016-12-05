use std::{u8, u32};
use std::path::PathBuf;

use clap::ArgMatches;
use ::error::*;

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
    pub fn from_cli(cli: ArgMatches) -> Result<Config> {
        Ok(Config {
            device: Config::get_device_from_cli(&cli)?,
            command: Config::get_command_from_cli(&cli)?,
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
    fn get_device_from_cli(cli: &ArgMatches) -> Result<Option<(u8, u8)>> {
        Ok(match cli.value_of("device") {
            Some(device_str) => {
                let mut split = device_str.split(":");
                let bus = split.next();
                let addr = split.next();
                if let (Some(bus), Some(addr), None) = (bus, addr, split.next()) {
                    Some((bus.parse()
                              .chain_err(|| {
                                  ErrorKind::CLI(format!("bus number must be an integeer between \
                                                          0 and {}",
                                                         u8::MAX))
                              })?,
                          addr.parse()
                              .chain_err(|| {
                                  ErrorKind::CLI(format!("device address must be an integeer \
                                                          between 0 and {}",
                                                         u8::MAX))
                              })?))
                } else {
                    return Err(Error::from_kind(ErrorKind::CLI("Device must be in \
                                                                `bus:addr` format"
                        .to_owned())));
                }
            }
            None => None,
        })
    }

    /// Gets the command used in te CLI.
    fn get_command_from_cli(cli: &ArgMatches) -> Result<Option<Command>> {
        if let Some(dump) = cli.subcommand_matches("dump") {
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
                    }.chain_err(|| {
                        ErrorKind::CLI(format!("memory address must be an integer from \
                                                0x00000000 to {:#010x}",
                                               u32::MAX))
                    })?;
                let size = if let Some(size_str) = dump.value_of("size") {
                    let size = if size_str.starts_with("0x") {
                            u32::from_str_radix(size_str.trim_left_matches("0x"), 16)
                        } else {
                            u32::from_str_radix(size_str, 10)
                        }.chain_err(|| {
                            ErrorKind::CLI(format!("dump size must be an integer from 0x00000000 \
                                                    to {:#010x} (the maximum size starting from \
                                                    the given address)",
                                                   u32::MAX - addr + 1))
                        })?;
                    if size > u32::MAX - addr + 1 {
                        return Err(Error::from_kind(ErrorKind::CLI(format!("dump size must be \
                                                                            an integer from \
                                                                            0x00000000 to \
                                                                            {:#010x} (the \
                                                                            maximum size \
                                                                            starting from the \
                                                                            given address)",
                                                                           u32::MAX - addr + 1))));
                    }
                    Some(size)
                } else {
                    None
                };
                Ok(Some(Command::Dump {
                    address: Some(addr),
                    size: size,
                    hex: dump.is_present("hex"),
                    sid: false,
                    out: if let Some(path) = dump.value_of("out") {
                        Some(PathBuf::from(path))
                    } else {
                        None
                    },
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
                    }.chain_err(|| {
                        ErrorKind::CLI(format!("memory address must be an integer from \
                                                0x00000000 to {:#010x}, given '{}'",
                                               u32::MAX,
                                               addr_str))
                    })?;
                let value_str = value_iter.next().unwrap();
                let word = if value_str.starts_with("0x") {
                    u32::from_str_radix(value_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(value_str, 10)
                };
                let final_value =
                    match word {
                        Ok(word) => {
                            if u32::MAX - 4 >= addr {
                                WriteData::Word(word)
                            } else {
                                let err_msg = format!("cannot write a complete word at address \
                                                       {:#010x}, it would write past the end of \
                                                       the memory address space \
                                                       (limit: {:#010x})",
                                                      addr,
                                                      u32::MAX);
                                return Err(Error::from_kind(ErrorKind::CLI(err_msg)));
                            }
                        }
                        Err(e) => {
                            let path = PathBuf::from(value_str);
                            if path.exists() {
                                let metadata = path.metadata()
                                    .chain_err(|| "could not read file metadata")?;
                                let max_bytes = (u32::MAX - addr + 1) as u64;
                                if metadata.len() > max_bytes {
                                    let err_msg = format!("the file '{}' is too big. The maximum \
                                                           file size to write to address \
                                                           {:#010x} is {} bytes, but the file \
                                                           had {} bytes",
                                                          path.display(),
                                                          addr,
                                                          max_bytes,
                                                          metadata.len());
                                    return Err(Error::from_kind(ErrorKind::CLI(err_msg)));
                                }
                                WriteData::File(Box::new(path))
                            } else {
                                return Err(Error::from_kind(ErrorKind::CLI(
                                format!("the file '{}' does not exist.\n\
                                        Note: If you were trying to provide a value, the integeer \
                                        conversion failed with this error: {}",
                                                                               path.display(),
                                                                               e))));
                            }
                        }
                    };
                addresses.push(addr);
                data.push(final_value);
            }
            Ok(Some(Command::Write {
                addresses: addresses,
                data: data,
            }))
        } else if let Some(exec) = cli.subcommand_matches("exec") {
            let addr_str = exec.value_of("addr").unwrap();
            let addr = if addr_str.starts_with("0x") {
                    u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(addr_str, 10)
                }.chain_err(|| {
                    ErrorKind::CLI(format!("memory address must be an integer from \
                                            0x00000000 to {:#010x}, given '{}'",
                                           u32::MAX,
                                           addr_str))
                })?;
            Ok(Some(Command::Execute { address: addr }))
        } else if let Some(reset64) = cli.subcommand_matches("reset64") {
            let addr_str = reset64.value_of("addr").unwrap();
            let addr = if addr_str.starts_with("0x") {
                    u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(addr_str, 10)
                }.chain_err(|| {
                    ErrorKind::CLI(format!("memory address must be an integer from \
                                            0x00000000 to {:#010x}, given '{}'",
                                           u32::MAX,
                                           addr_str))
                })?;
            Ok(Some(Command::Reset64 { address: addr }))
        } else if cli.subcommand_matches("version").is_some() {
            Ok(Some(Command::Version))
        } else if let Some(clear) = cli.subcommand_matches("clear") {
            let addr_str = clear.value_of("addr").unwrap();
            let addr = if addr_str.starts_with("0x") {
                    u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(addr_str, 10)
                }.chain_err(|| {
                    ErrorKind::CLI(format!("memory address must be an integer from \
                                            0x00000000 to {:#010x}, given '{}'",
                                           u32::MAX,
                                           addr_str))
                })?;
            let num_bytes_str = clear.value_of("num_bytes").unwrap();
            let num_bytes = if num_bytes_str.starts_with("0x") {
                    u32::from_str_radix(num_bytes_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(num_bytes_str, 10)
                }.chain_err(|| {
                    ErrorKind::CLI(format!("the number of bytes to clear must be an integer from \
                                            0x00000000 to {:#010x} (the maximum size starting \
                                            from the given address)",
                                           u32::MAX - addr + 1))
                })?;
            if num_bytes > u32::MAX - addr + 1 {
                return Err(Error::from_kind(ErrorKind::CLI(format!("clear size must be an \
                                                                    integer from 0x00000000 \
                                                                    to {:#010x} (the maximum \
                                                                    size starting from the \
                                                                    given address)",
                                                                   u32::MAX - addr + 1))));
            }

            Ok(Some(Command::Clear {
                address: addr,
                num_bytes: num_bytes,
            }))
        } else if let Some(fill) = cli.subcommand_matches("fill") {
            let addr_str = fill.value_of("addr").unwrap();
            let addr = if addr_str.starts_with("0x") {
                    u32::from_str_radix(addr_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(addr_str, 10)
                }.chain_err(|| {
                    ErrorKind::CLI(format!("memory address must be an integer from \
                                            0x00000000 to {:#010x}, given '{}'",
                                           u32::MAX,
                                           addr_str))
                })?;
            let num_bytes_str = fill.value_of("num_bytes").unwrap();
            let num_bytes = if num_bytes_str.starts_with("0x") {
                    u32::from_str_radix(num_bytes_str.trim_left_matches("0x"), 16)
                } else {
                    u32::from_str_radix(num_bytes_str, 10)
                }.chain_err(|| {
                    ErrorKind::CLI(format!("the number of bytes to fill must be an integer from \
                                            0x00000000 to {:#010x} (the maximum size starting \
                                            from the given address)",
                                           u32::MAX - addr + 1))
                })?;
            let fill_byte_str = fill.value_of("fill_byte").unwrap();
            let fill_byte = if fill_byte_str.starts_with("0x") {
                    u8::from_str_radix(fill_byte_str.trim_left_matches("0x"), 16)
                } else {
                    u8::from_str_radix(fill_byte_str, 10)
                }.chain_err(|| {
                    ErrorKind::CLI(format!("the filling byte must be an integer from 0x00 to \
                                            {:#04x}, given '{}'",
                                           u8::MAX,
                                           fill_byte_str))
                })?;
            Ok(Some(Command::Fill {
                address: addr,
                num_bytes: num_bytes,
                fill_byte: fill_byte,
            }))
        } else {
            Ok(None)
        }
    }
}
