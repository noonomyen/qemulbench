#!/bin/sh

trap "sync; poweroff -f" EXIT INT TERM HUP

mount -t proc none /proc 2>/dev/null
mount -t sysfs none /sys 2>/dev/null
mount -t devtmpfs none /dev 2>/dev/null
mkdir -p /dev/pts /dev/shm
mount -t devpts none /dev/pts 2>/dev/null
mount -t tmpfs none /dev/shm 2>/dev/null
mount -t tmpfs none /tmp 2>/dev/null

export PATH=/bin:/sbin:/usr/bin:/usr/sbin
export HOME=/root
export USER=root
export TERM=xterm-256color

# Intercept reboot and halt commands to always cleanly power off
rm -f /sbin/reboot /sbin/halt /bin/reboot /bin/halt 2>/dev/null
ln -sf poweroff /sbin/reboot 2>/dev/null || true
ln -sf poweroff /sbin/halt 2>/dev/null || true

# Parse kernel command line
CMD=""
for param in $(cat /proc/cmdline); do
    case "$param" in
        qemumount64=*)
            MNT_B64="${param#qemumount64=}"
            MNT_LIST="$(echo "$MNT_B64" | base64 -d 2>/dev/null)"
            for pair in $MNT_LIST; do
                TAG="${pair%%:*}"
                TARGET="${pair#*:}"
                if [ -n "$TAG" ] && [ -n "$TARGET" ]; then
                    mkdir -p "$TARGET" 2>/dev/null
                    mount -t 9p -o trans=virtio,version=9p2000.L "$TAG" "$TARGET" 2>/dev/null || true
                fi
            done
            ;;
        qemucmd64=*)
            B64="${param#qemucmd64=}"
            CMD="$(echo "$B64" | base64 -d 2>/dev/null)"
            ;;
        qemucmd=*)
            if [ -z "$CMD" ]; then
                CMD="${param#qemucmd=}"
            fi
            ;;
    esac
done

if [ -n "$CMD" ] && [ "$CMD" != "/bin/sh" ] && [ "$CMD" != "sh" ]; then
    set +e
    ( sh -c "$CMD" )
    RET=$?
    sync
    echo "[qemulbench_exit_code=$RET]"
    poweroff -f
fi

setsid cttyhack /bin/sh </dev/console >/dev/console 2>&1 || /bin/sh </dev/console >/dev/console 2>&1
sync
poweroff -f
while true; do sleep 1; done
