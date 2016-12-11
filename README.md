# CLI tools for Allwinner devices in FEL mode

Tools based in [sunxi-tools](sunxi-tools) for using Allwinner devices in FEL
mode.

## Usage

```text
USAGE:
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
    spl        Loads and executes U-Boot SPL. If file additionally contains a
               main U-Boot binary, it will transfer it to memory and print the
               entry point address, in hex
    version    Gets SoC version information
    write      Write data to device memory
```

## Porting from sunxi-tools

The CLI of `fel-cli` is almost a drop-in replacement for the `sunxi-fel` command,
but there are some changes that are needed.

 - You cannot use more than one sub-command at once.
 - There is no `--verbose` option (yet).
 - The `--progress` option is not (yet) supported.
 - The `hexdump` subcommand has to be updated to `dump --hex`. The output will
   be exactly the same.
 - `ver[sion]` option cannot be specified as `ver`. Must be complete as
   `version`.
 - No `read` or `readl` commands. Those have been added to the `dump` command.
   By default, the `dump` command will dump one 32-bit word in `0x00000000`
   format. If you add the `<size>` argument, you can dump that number of bytes.
   By default, it will output the data to *stdout*, but if you don't want it in
   hexadecimal (as said before, with the `--hex` flag) you can output it to a
   file by using the `-o | --out` option.
 - No `sid` command, it has been added to the `dump` command, and can be used
   with `dump --sid`.
 - The `-d | --dev` option has been changed in long mode to `--device`. So if
   you use `-d`, no change is needed, but if you use `--dev`, you will need to
   change it to `--device`.
 - No `uboot` option. If you want the U-Boot binary to be executed, you can add
   the `-x` or the `--exec` flag. If you want to concatenate more than one
   subcommand, the U-Boot entry address is printed after SPL loading (if no `-x`
   was supplied). You can then use the `exec` submenu to execute the U-Boot at
   that address.
 - `exe[cute]` submenu has been modified to `exec`.
 - No `[x]gauge` output options (yet?).
 - `multi[write]` is now integrated in the `write` command. Simply add more\
   words or files to the list: `fel-cli write address1 file1 address2 word1 ...`.
 - No `writel` command. You can use 32-bit words as well as files in the `write`
   command. Just make sure that you have no file named with an integer (both in
   hex or in decimal).

The rest of the options should work the same way. If not, please, fill an issue.

## License

This code is distributed under the terms of both the MIT license and the Apache
License (Version 2.0). See LICENSE-APACHE, and LICENSE-MIT for details.

[sunxi-tools]: https://github.com/linux-sunxi/sunxi-tools
