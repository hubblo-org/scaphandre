#!/bin/bash

whoami=$(whoami)

sudo chown root:${whoami} -R /sys/devices/virtual/powercap/*
sudo chmod g+r -R /sys/devices/virtual/powercap/*