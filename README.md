# anyrun-provider

The backend of [Anyrun](https://github.com/anyrun-org/anyrun),
loads Anyrun plugins and is used as the middleman between launchers and the plugins.

## Usage

For integrating this into other applications, you can use the `anyrun-provider-ipc` crate available
in this repo. It contains a simple socket implementation, and the reference for the data types
used in the communication. The communication socket can either be managed by your program, or
`anyrun-provider`, depending on the command line arguments provided.

For a reference implementation of how this is used, refer to [provider.rs](https://github.com/anyrun-org/anyrun/blob/anyrun-provider/anyrun/src/provider.rs).
