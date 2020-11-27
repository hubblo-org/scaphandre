Since linux kernel package 5.4.0-53.59 in debian/ubuntu, powercap attributes are only accessible by root:

    linux (5.4.0-53.59) focal; urgency=medium

      * CVE-2020-8694
        - powercap: make attributes only readable by root

Therefor, scaphandre needs to ensure that its user has access to /sys/class/powercap.