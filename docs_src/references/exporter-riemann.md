# Riemann exporter

![riemann exporter](images/riemann_exporter.png)

## Usage

You can launch the Riemann exporter this way (running the default powercap_rapl sensor):

	scaphandre riemann

As always exporter's options can be displayed with `-h`:
```
scaphandre-riemann
Riemann exporter sends power consumption metrics to a Riemann server

USAGE:
    scaphandre riemann [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
        --mtls       Connect to a Riemann server using mTLS. Parameters address, ca, cert and key must be defined.
    -q, --qemu       Instruct that scaphandre is running on an hypervisor
    -V, --version    Prints version information

OPTIONS:
    -a, --address <address>               Riemann ipv6 or ipv4 address. If mTLS is used then server fqdn must be
                                          provided [default: localhost]
    -d, --dispatch <dispatch_duration>    Duration between metrics dispatch [default: 5]
    -p, --port <port>                     Riemann TCP port number [default: 5555]
        --ca <cafile>                     CA certificate file (.pem format)
        --cert <certfile>                 Client certificate file (.pem format)
        --key <keyfile>                   Client RSA key
```
With default options values, the metrics are sent to http://localhost:5555 every 5 seconds

Use `--mtls` option to connect to a Riemann server using mTLS. In such case, you must provide the following parameters:
* `--address` to specify the **fqdn** of the Riemann server.
* `--ca` to specify the CA that authenticate the Riemann server.
* `--cert` to specify the client certificate.
* `--key` to specify the **RSA** key to be used by the client certificate.

Use `-q` or `--qemu` option if you are running scaphandre on a hypervisor. In that case a label with the vm name will be added to all `qemu-system*` processes.
This will allow to easily create charts consumption for each vm and defined which one is the top contributor.

*Troubleshooting note:* run  Scaphandre using `-vv` parameter. If Scaphandre is stuck on the `Send data` log event, ensure you are connecting the Riemann server using a TLS port (5554 in the below example).
As a reference here is a Riemann configuration:
```
; -*- mode: clojure; -*-
; vim: filetype=clojure

(logging/init {:file "riemann.log"})

; Listen on the local interface over TCP (5555), UDP (5555), TLS/TCP (5554)  and websockets
; (5556)
(let [host "0.0.0.0"]
  (tcp-server {:host host})
  (tcp-server {:host host
               :port 5554
               :tls? true
               :key "/client.key.pkcs8"
               :cert "/client.pem"
               :ca-cert "/CA.pem"})
  (udp-server {:host host})
  (ws-server  {:host host}))

; Expire old events from the index every 5 seconds.
(periodically-expire 5)

(let [index (index)]
  ; Inbound events will be passed to these streams:
  (streams
    (default :ttl 60
      ; Index all events immediately.
      index

      ; Log expired events.
      (expired
        (fn [event] (info "expired" event))))))
```

## Metrics exposed
Typically the Riemann exporter is working in the same way as the prometheus exporter regarding metrics. Please look at details in [Prometheus exporter](exporter-prometheus.md) documentations.

There is only one exception about `process_power_consumption_microwatts` each process has a service name `process_power_consumption_microwatts_pid_exe`.

As an example, process consumption can be retrieved using the following Riemann query:
```
(service =~ "process_power_consumption_microwatts_%_firefox") or (service =~ "process_power_consumption_microwatts_%_scaphandre")
```
