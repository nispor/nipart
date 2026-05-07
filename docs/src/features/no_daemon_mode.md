# No Daemon mode

When passing with `--no-daemon` option in `npt apply <YAML_FILE>`, 
`npt` command will not contact nipartd daemon but apply the configuration
directly to kernel or related daemon(e.g. OpenvSwitch daemon).

Limitations:
 * Cannot renew DHCP lease. When you request DHCP in desired YAML file,
   `npt` will request DHCP lease and apply DHCP lease IP addresses with
   `preferred-life-time` and `valid-life-time` allowing kernel to purge the IP
   after lease expired. Hence you need to run
   `np apply <YAML_FILE> --no-daemon` on a regular basis to renew DHCP lease.

 * Does not support [conditional interface up down](./trigger.md) because
   we don't have daemon to monitor link carrier state and trigger interface
   up/down conditionally. Use daemon mode instead.
