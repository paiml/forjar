# CRUX survey — forjar#724: how other tools decide a target is "this host"

Question: when a declared target IS the controller, how does each tool decide
to skip its remote transport, and would a host declared by its own static
overlay address (10.42.0.20) be run locally?

Recorded from documentation memory, not re-measured against each tool on this
host; treat each line as cited, not measured.

- Ansible — `connection: local`, or the implicit `localhost` (when `localhost`
  is not in inventory, Ansible synthesizes it with the local connection). A
  host listed by its own LAN or overlay IP goes over SSH unless the operator
  sets `ansible_connection=local`. Decided by explicit setting or spelling.
- Puppet Bolt — the `localhost` target uses the local transport by default; any
  other name or address uses SSH/WinRM unless configured `transport: local`.
- pyinfra — `@local` is the local connector; a bare host or IP is SSH.
- Salt-SSH — the roster entry is always reached over SSH; local execution is
  `salt-call --local` on the minion, a different command, not a roster decision.

Verdict: none of the four infers "local" from the interfaces the controller
holds — each needs an explicit setting or a loopback spelling. forjar's fix goes
further than the field: it asks the kernel whether this host holds the address
(bind + connect on Linux), and declines to SSH (the field's behaviour) wherever
it cannot answer — off Linux, with ip_nonlocal_bind set or unreadable, and for
IPv6 link-local. The cost of going further is the bypass named in the module
doc: a held address skips any ssh-config remap and the machine's user/ssh_key,
the same trust forjar already extends to loopback spellings and /etc/hostname.
An operator who wants SSH to self can still get it by declaring a name that
does not match /etc/hostname and resolving it elsewhere; nothing in the field
offers a "local by interface" switch to copy instead.
