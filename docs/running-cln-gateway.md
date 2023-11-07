# Running a Gateway with Core Lightning

We've made a single repo that contains all the binariesfor the gateway, the core lightning plugin, and the core lightning daemon. This makes it easy to run the gateway with core lightning. You can just clone the repo, run 1 install script, and run "just run" to have a core lightning node with fedimint gateway running on your machine.

Otherwise if you want to set it up yourself step by step...

## Installing Core Lightning

As always, the correct way to install your software is with nix. If you don't have nix installed, you can install it with the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

Then you can install core lightning from the nix package registry:

```bash
nix-env -iA nixpkgs.clightning
```

Then confirm the install (should return v23.05)

```bash
lightningd --version
```

Finally, create a `.lightning` directory locally or in your home directory (`~/.lightning`) to store the lightningd data, and a plugins directory inside of that.

We'll also create a `.gateway` directory to store the gateway data for when we run it.

```bash
mkdir .lightning
mkdir .lightning/plugins
mkdir .gateway
```

## Setting up the Mutinynet Esplora Backend

We're going to run the coreln gateway on mutinynet because it's awesome and has covenants. We'll use fiatjaf's `trustedcoin` plugin to use an esplora backend for the bitcoin node, and point that esplora backend at `mutinynet.com`. You can also run your own esplora backend if you want or point it at other networks, but this is the easiest way to get started.

### Install the Trustedcoin Plugin and Modify it for Mutinynet

Clone Trustedcoin. (You'll need go installed, which you can install via nix with `nix-env -iA nixpkgs.go`)

```bash
git clone https://github.com/nbd-wtf/trustedcoin
```

To run it on mutinynet, you'll just have to change the signet esplora url in `main.go` to "mutinynet.com/api"

```diff
		"signet": {
-			"https://mempool.space/signet/api",
+			"https://mutinynet.com/api",
		},
```

Then build the plugin

```bash
cd trustedcoin && go build
```

This will create a binary called `trustedcoin` in the current directory. Move this binary to your `.lightning/plugins` directory wherever you made it

```bash
mv trustedcoin ../.lightning/plugins
```

You can then delete the trustedcoin directory, we don't need it anymore.

```bash
cd .. && rm -rf trustedcoin
```

You can now run the core lightning node on mainnet, testnet, or mutinynet using the esplora backend instead of a bitcoin node. Try running it with:

```bash
lightningd --lightning-dir=./.lightning --signet --disable-plugin=bcli
```

## Installing the Fedimint Gateway Daemon and CLN Plugin

Clone the fedimint repo

```bash
git clone https://github.com/fedimint/fedimint
```

Enter the nix flake shell to get all the fedimint dependencies for rust/cargo etc.

```bash
cd fedimint && nix develop
```

Then to exclusively build the gateway binaries we want (gatewayd, gateway-cli, and the core lightning plugin), run the following commands

```bash
cargo build --release --bin gateway-cln-extension
cargo build --release --bin gatewayd
cargo build --release --bin gateway-cli
```

This will create 3 binaries in the `target/release` directory. Let's copy the gatewayd and cli to our local dir where we're doing everything.then you can also move the gateway binaries to your bin directory (or anywhere else in your path). The cln plugin we'll move to the plugins directory we made earlier.

```bash
cp target/release/gatewayd ../
cp target/release/gateway-cli ../
cp target/release/gateway-cln-extension ../.lightning/plugins

cp target/release/gateway-cln-extension ~/.local/bin
cp target/release/gatewayd ~/.local/bin

# check that they're in your path now
which gatewayd
which gateway-cli
```

You can then delete the fedimint directory, we don't need it anymore.

```bash
cd .. && rm -rf fedimint
```

## Running the Gateway with Core Lightning

You should now have the gatewayd and gateway-cli binaries in your path, and the trustedcoin and gateway-cln-extension plugins in your `.lightning/plugins` directory. You can now start the lightning node and gatewayd!

To run coreln with the gateway, you have to specify the `--fm-gateway-listen` option. This option takes a `host:port` and will listen on the specified host and port for incoming connections from the gateway. Make sure to use the same port when you specify the gateway config.

```bash
lightningd --lightning-dir=./.lightning --signet --disable-plugin=bcli --fm-gateway-listen "127.0.0.1:3301"
```

Then you can run the gatewayd daemon with the same host and port, and use the `.gateway` directory we made earlier to store the gateway data. You'll also need some other configs. You can see all the configs with `gatewayd --help` but here's a quick example (password can be whatever you want):

```bash
gatewayd --data-dir=./.gateway --network=signet --listen="127.0.0.1:3301" --api-addr="127.0.0.1:3300" --password "thereisnosecondbest" cln --cln-extension-addr "https://127.0.0.1:3302"
```

You can also set all of those as environment variables in a .env if you want:

```bash
FM_GATEWAY_DATA_DIR=
FM_GATEWAY_LISTEN_ADDR=
FM_GATEWAY_API_ADDR=
FM_GATEWAY_PASSWORD=
FM_GATEWAY_NETWORK=
FM_GATEWAY_FEES=
FM_NUMBER_OF_ROUTE_HINTS=
```

Now we can interaction with the gateway and the cln node using the cli tools, which we can alias to:

```bash
alias cln-cli="lightning-cli --lightning-dir=./.lightning --signet"
alias gw-cli='gateway-cli --data-dir ./.gateway --network signet --password "thereisnosecondbest"'
```

Let's also put those commands in a `JUSTFILE` so we can run them with `just run`:

```bash
run:
    just lightning & just gateway

lightning:
    lightningd --lightning-dir=./.lightning --signet --disable-plugin=bcli --fm-gateway-listen "127.0.0.1:3300"

gateway:
    gatewayd --data-dir=./.gateway --network=signet --listen="127.0.0.1:3300" --api-addr="https://127.0.0.1:3301" --password "thereisnosecondbest" cln --cln-extension-addr "https://127.0.0.1:3302"

alias:
    alias cln-cli="lightning-cli --lightning-dir=./.lightning --signet && alias gw-cli='gateway-cli --data-dir ./.gateway --network signet --password "thereisnosecondbest"'

```

Now you can run `just run` to run both the lightning node and the gatewayd daemon, or you can run `just lightning` or `just gateway` to run them individually. You can also run `just alias` to set the aliases for the cli tools.
