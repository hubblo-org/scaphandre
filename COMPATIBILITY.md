Since linux kernel package 5.4.0-53.59 in debian/ubuntu, powercap attributes are only accessible by root:

    linux (5.4.0-53.59) focal; urgency=medium

      * CVE-2020-8694
        - powercap: make attributes only readable by root

Therefor, scaphandre needs to ensure that its user has access to /sys/class/powercap.

For AMD processors, it seems that powercap/rapl will work only since kernel 5.8: [https://www.phoronix.com/scan.php?page=news_item&px=Google-Zen-RAPL-PowerCap](https://www.phoronix.com/scan.php?page=news_item&px=Google-Zen-RAPL-PowerCap)
and 5.11 for family 19h: [https://www.phoronix.com/scan.php?page=news_item&px=AMD-RAPL-Linux-Now-19h](https://www.phoronix.com/scan.php?page=news_item&px=AMD-RAPL-Linux-Now-19h)
