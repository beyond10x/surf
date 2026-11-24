#!/bin/sh
# Runs a command in a user and network namespace of its own, with only loopback up: the page
# step's Playwright call, with the browser and every surf-view-fixture it starts
# (story:view-live-marker-under-load).
#
# usage: page-netns.sh <command> [argument...]
#
# Chromium cancels every request in flight with net::ERR_NETWORK_CHANGED when an address of any
# interface of its network namespace is added or removed, a request to loopback included. On a
# host where containers start and stop, a page's module files were cancelled so, and its script
# never ran: the live marker stayed `connecting`. In a namespace of its own the browser sees no
# change of the host's interfaces, and the step reaches nothing but loopback.
#
# The namespace needs unprivileged user namespaces (unshare --user --map-root-user --net) and
# `ip`. Without them the script prints one line on standard error, starting with "page step: no
# network namespace of its own", and exits 1: the tests never run outside a namespace. The
# forge's runner allows them in a step of .github/workflows/check.yml.
set -u

if ! reason=$(unshare --user --map-root-user --net ip link set lo up 2>&1); then
  newline='
'
  printf '%s (%s): %s\n' "page step: no network namespace of its own" \
    "unshare --user --map-root-user --net, ip link set lo up" "${reason%%"$newline"*}" >&2
  exit 1
fi
exec unshare --user --map-root-user --net sh -c 'ip link set lo up && exec "$@"' page-netns "$@"
