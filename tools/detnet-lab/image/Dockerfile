# Node image for the DetNet testbed. Binaries (zenohd, zprobe) are built on the
# host and bind-mounted at /opt/zenoh, so the base must match the host glibc.
FROM archlinux:latest
RUN pacman -Sy --noconfirm --needed iproute2 iperf3 procps-ng ethtool && \
    pacman -Scc --noconfirm
CMD ["sleep", "infinity"]
