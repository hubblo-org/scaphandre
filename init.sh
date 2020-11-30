#!/bin/bash

WHOAMI=$(whoami)

for i in $(find /sys/devices/virtual/powercap -name energy_uj)
do
  sudo chown root:${WHOAMI} ${i}
  sudo chmod g+r -R ${i}
done
