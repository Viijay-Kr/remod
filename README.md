# Remod

> Codemods are transformations that run on your codebase programmatically. This allows a large number of changes to be programmatically applied without having to manually go through every file.

`React Codemod` powered by Rust/[SWC](https://swc.rs/)

## Tramsformations (Currently Supported)
* [Display names](https://legacy.reactjs.org/docs/react-component.html#displayname)
  * Add display name
  * Rename display name
  * Delete display names
* Storybook
  * Create story files for components

## Installation 

### Mac OS
TBD

### Linux
TBD

### Windows
TBD

### Build from source
#### Prequisites
* Rust
* Rustup
* Cargo

#### Clone
`git clone repo`

#### Build
`cargo build`

> By default the workspace is configured to build remod_cli

The binary should be built in `/target/debug/remod_cli`
 
Run `remod_cli --h` to see the list of available commands and their usage

#### Development
`cargo run`