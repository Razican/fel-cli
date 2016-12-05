# CLI tools for Allwinner devices in FEL mode

Tools based in sunxi-tools for using Allwinner devices in FEL mode.

## Usage

```text
fel-cli [OPTIONS] [SUBCOMMAND]

FLAGS:
 -h, --help       Prints help information
 -V, --version    Prints version information

OPTIONS:
 -d, --device <bus:addr>    The USB bus and device address of the FEL device

SUBCOMMANDS:
 clear      Clear memory
 dump       Dumps memory region in binary through stdout
 exec       Call function at the given address
 fill       Fill memory with the given byte
 help       Prints this message or the help of the given subcommand(s)
 reset64    RMR request for AArch64 warm boot
 version    Gets SoC version information
 write      Write data to device memory
```

## License ##

This code is distributed under the terms of both the MIT license and the Apache
License (Version 2.0). See LICENSE-APACHE, and LICENSE-MIT for details.
