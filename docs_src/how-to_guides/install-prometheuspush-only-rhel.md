# Install Scaphandre with only Prometheus-push exporter compiled, for Prometheus Push Gateway, on RHEL 8 and 9

## Manual installation

Scaphandre can be compiled with a limited set of features. You have the choice to only install Scaphandre with prometheus-push exporter (alongside with stdout and json exporters, which might be useful locally).

RPM packages containing only those features are provided for RHEL 8 and 9 :
- [RPM package for RHEL8](https://scaphandre.s3.fr-par.scw.cloud/x86_64/scaphandre-prometheuspush-dev0.5.10-1.el8.x86_64.rpm)
- [RPM package for RHEL9](https://scaphandre.s3.fr-par.scw.cloud/x86_64/scaphandre-prometheuspush-dev0.5.10-1.el9.x86_64.rpm)

You can download it and install it just providing the right URL to dnf :

    dnf install -y URL

Then you'll probably need to change its configuration to target the appropriate Push Gateway server. Edit the configuration file :

    vi /etc/scaphandre/prometheuspush

Default options look like :

    SCAPHANDRE_ARGS="prometheus-push -H localhost -S http"

Those are prometheus-push exporter CLI options. Run the executable to get the reference of the options :

    /usr/bin/scaphandre-prometheuspush --help

A simple configuration to target Push Gateway reachable on https://myserver.mydomain:PORT would look like :

    SCAPHANDRE_ARGS="prometheus-push -H myserver.mydomain -S https -p PORT"

Once the configuration is changed, you can restart the service and ensure it is enabled as well for next reboot :

    systemctl restart scaphandre-prometheuspush && systemctl enable scaphandre-prometheuspush

Configuration issues or issues to reach the push gateway should be visible in the logs :

    systemctl status scaphandre-prometheuspush