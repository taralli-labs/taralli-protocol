# Taralli Auction

## Test

Currently we have one end-to-end test at `src/bin/server.rs` which requires:
1. the verifier server running
2. anvil running
3. the env vars `CONTRACT_ADDRESS` and `ADMIN_ADDRESS` containing addresses to the contract and its owner (see [how to deploy the contract](../taralli-ledger-client/contracts/README.md#deploy))
4. funds at the address `0x70997970C51812dc3A010C7d01b50e0d17dc79C8` (though feel free to make this configurable)

```bash
export CONTRACT_ADDRESS=
export ADMIN_ADDRESS=
# run from crates/taralli-auction
cargo test -- --nocapture
```

## Run NGINX Locally
First of, you'll need to download nginx, see the [docs](https://docs.nginx.com/nginx/admin-guide/installing-nginx/installing-nginx-open-source/).

### Placing configs
That being done, you'll need to copy this repo's config to `etc`.

```bash
sudo cp ./deployment/nginx/nginx.conf /etc/nginx/conf.d/taralli-auth.conf
```

Here, we place said file under `conf.d` as `taralli-auth.conf`. You might also need to change nginx's default configs. Go to `etc/nginx/nginx.conf.default` find the only non commented `server` definition and change its http/https ports to `8081`, which is also what we use within the cp'd file.

Run this to make sure your file is ok:

```bash
sudo nginx -t
```

To then start, stop, restart nginx, check the [docs](https://linuxize.com/post/start-stop-restart-nginx/).

Some troubleshooting commands, in case nginx is unable to start on the given port:
```bash
sudo iptables -A INPUT -p tcp --dport 8081 -j ACCEPT
sudo setenforce 0
```

### Running
We can use our examples (`cargo run --bin server`, `cargo run --example simple_request`) to validate the rate limiting. Mind you, I have yet to inser the logic to direct said example's reqs to port 8081. If you don't do it, requests will flow just fine.