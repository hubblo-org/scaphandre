# About containers

Let's first define what you want to achieve. Do you want to:

1. run scaphandre **in a container**, to not have to manage the dependencies, then measure the power consumption of a **bare metal host** ?
2. measure the **power consumption of containers** running on a host ?
3. measure the power consumption of a distributed **container orchestrator** (like [kubernetes]()) and the applications running on it ?

Use case **1** is described [here](../tutorials/in-container.md), **2** has a dedicated [how-to](../how-to_guides/measure-containers-power.md), **3** is described [here](../tutorials/kubernetes.md).

As described [here](../compatibility.md), scaphandre provides several ways ([sensors](../explanations/sensors.md)) to collect the power consumption metrics. Depending on your use case a sensor should be more suitable than the other. Each of them comes with strengths and weaknesses. This is basically always a tradeoff between precision and simplicity.