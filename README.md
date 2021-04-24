# HIDtoVPAD Network Client - Command Line Edition

HIDtoVPAD network client focused on working only through the command line.

Written in Rust for use in low-memory deployments, such as in a Raspberry Pi.

## Features

- Predictable latency
- Configurable controller polling rate
- Low memory footprint (usually less than 2MB)
- Easy to use
- No need to add new mappings on the WiiU

## Details

- Supports only XInput controllers on Windows

## How to use

This client automatically attaches all controllers. If you want to detach a controller, you need to disconnect it from your computer.

```bash
# ./network-client --help
Command line HIDtoVPAD network client v1.0.0

USAGE:
    network-client [OPTIONS] <ip>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --polling-rate <polling-rate>    Sets a custom polling rate. Must be between 20 and 1000 Hz. [default: 250]

ARGS:
    <ip>    Sets the IP address to connect, for example 192.168.2.3
```

## Creating mappings

Usually not necessary, but if needed mappings can be created with [SDL2 Gamepad Tool](https://www.generalarcade.com/gamepadtool/).
To use them one just need to set the environment variable `SDL_GAMECONTROLLERCONFIG` with the generated mapping before loading the application.

### Example

```bash
export SDL_GAMECONTROLLERCONFIG="030000005e040000120b000005050000,XBox Series Controller,a:b0,b:b1,x:b2,y:b3,back:b6,guide:b8,start:b7,leftstick:b9,rightstick:b10,leftshoulder:b4,rightshoulder:b5,dpup:h0.1,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,leftx:a0,lefty:a1,rightx:a3,righty:a4,lefttrigger:a2,righttrigger:a5,platform:Linux,"
./network-client 192.168.1.2
```
