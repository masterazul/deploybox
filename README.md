# deploybox

Spin up a throwaway preview of any repo with one command. `deploybox` builds the
Dockerfile, runs it on a free port, tracks it, and can reverse-proxy every preview under
a single host — a tiny self-hosted take on preview deployments.

It shells out to the `docker` CLI you already have; there is no daemon and no SDK.

## Build

```
cargo build --release
```

## Use

```
deploybox up examples/app --name hello --container-port 8000
#  preview 'hello' is up (a1b2c3d4e5f6)
#    direct:  http://127.0.0.1:8123
#    proxied: http://hello.localhost:8080

deploybox ls
deploybox proxy            # routes http://<name>.localhost:8080 to each preview
deploybox down hello
```

Run an existing image instead of building:

```
deploybox up --image traefik/whoami --name who --container-port 80
```

## How it works

- `up` runs `docker build`/`docker run -p 127.0.0.1:<free>:<container-port>`, labels the
  container, and records it in `~/.deploybox/state.json`.
- `proxy` reads that state and routes by the first label of the `Host` header
  (`hello.localhost` → the `hello` preview). `*.localhost` already resolves to 127.0.0.1.
- `down` removes the container and the record.

The bundled proxy is intentionally small (HTTP/1.1, one request per connection, forces
`Connection: close`); put a real proxy in front for production traffic.

## License

MIT
