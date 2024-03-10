## How to launch a local network?

### Build parachain binary

Use the following command to build the node without launching it:

```sh
cargo build --release
```

### Get Polkadot Binaries

Next you will need a compatible release of [Polkadot](https://github.com/paritytech/polkadot-sdk) to run a testnet.
```sh
cd polkadot
cargo build --release
```
You'll find "polkadot", "polkadot-execute-worker" and "polkadot-prepare-worker" three binaries. Copy them to this project fold under ./bin directory.

### Download Zombienet executable

 You may also want to use [Zombienet (available for Linux and MacOS)](https://github.com/paritytech/zombienet/releases) for spinning up a testnet: 


You can find linux executables of the Zombienet CLI here:

https://github.com/paritytech/zombienet/releases
Download the Zombienet CLI according to your operating system.

Tip: download the executable to your working directory:
```sh
wget https://github.com/paritytech/zombienet/releases/download/v1.3.94/zombienet-linux-x64
chmod +x zombienet-linux-x64
```
Make sure Zombienet CLI is installed correctly:
```sh
./zombienet-linux-x64 --help
```
You should see some similar output:
```sh
Usage: zombienet [options] [command]

Options:
  -c, --spawn-concurrency <concurrency>  Number of concurrent spawning process to launch, default is 1
  -p, --provider <provider>              Override provider to use (choices: "podman", "kubernetes", "native")
  -m, --monitor                          Start as monitor, do not auto cleanup network
  -h, --help                             display help for command

Commands:
  spawn <networkConfig> [creds]          Spawn the network defined in the config
  test <testFile> [runningNetworkSpec]   Run tests on the network defined
  setup <binaries...>                    Setup is meant for downloading and making dev environment of Zombienet ready
  version                                Prints zombienet version
  help [command]                         display help for command

```

### Checklist before you launch:
1. Under the bin folder in your project directory, there should be three binary files: "polkadot", "polkadot-execute-worker", and "polkadot-prepare-worker".
2. In the project directory, a binary file named "zombienet-linux-x64" is downloaded.
3. "rococo-local-config.toml" file under the zombienet-config folder, of which parameters for the relay chain and parachain have already been adjusted.

### Command to launch Zombienet
```sh
./zombienet.sh spawn
```