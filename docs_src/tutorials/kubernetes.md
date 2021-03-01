# Kubernetes

This tutorial uses [Helm](https://helm.sh/docs/intro/quickstart/) to install
Scaphandre, Prometheus and Grafana.

## Install Scaphandre

First we install Scaphandre which runs as a daemon set which creates a pod on
each node for collecting the metrics.

    helm install scaphandre helm/scaphandre

## Install Prometheus

Next we will install Prometheus which will scrape the metrics generated by Scaphandre.

    helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
    helm repo add kube-state-metrics https://kubernetes.github.io/kube-state-metrics
    helm repo update

    helm install prometheus prometheus-community/prometheus \
    --set alertmanager.persistentVolume.enabled=false \
    --set server.persistentVolume.enabled=false

This setup should only be used for testing as the Prometheus data is not
persisted if the pods are deleted.

You can access the Prometheus web UI by creating a port forwarding connection.

    kubectl port-forward deploy/prometheus-server 9090:9090

## Install Grafana

Create a configmap to store the Grafana dashboard.

    kubectl create configmap scaphandre-dashboard \
        --from-file=scaphandre-dashboard.json=docs_src/tutorials/grafana-kubernetes-dashboard.json

Install Grafana.

    helm repo add grafana https://grafana.github.io/helm-charts
    helm repo update

    helm install grafana grafana/grafana --values docs_src/tutorials/grafana-helm-values.yaml

Get the Grafana web UI password which is randomly generated.

    kubectl get secret grafana -o jsonpath="{.data.admin-password}" | base64 --decode

Create a port forwarding connection to the Grafana pod.

    kubectl port-forward deploy/grafana 3000:3000

Open Grafana in your browser at http://localhost:3000 the username is admin.

## Cleaning up

Deleting the Helm releases will remove all the resources we created.

    helm delete grafana prometheus scaphandre