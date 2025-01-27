 * Checkpoint
 * /etc/nipart/conf.d/ for choosing which DHCP plugin to load
 * Native plugin should has its own log postfix
 * Wait online

```
wait-onlines:
 - global:
   - link
 - name: link
   type: link
   timeout: 30      # maximum wait for link up of interfaces
   post-sleep: 1    # sleep after link up of all interfaces before notify
   links:
     - all
 - name: arp
   type: arp
   addresses:
     - 192.168.1.1
 - name: nfs
   type: ping
   addresses:
     - 192.168.1.1
 - name: dns
   type: dns-resolve
   dns-reoslve:
     - www.example.com
```

 * Isolate secrets to separate file
 * Stealth(Minimum footprint) mode
 * DBUS API
 * NM1 Keyfile support
 * Instead process NipartEvent in `handle_event()`, we should provide
   default implementation of each event type as function in
   `NipartNativePlugin` and `NipartExternalPlugin`.
