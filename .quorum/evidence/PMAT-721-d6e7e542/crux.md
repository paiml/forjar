# Crux survey for epic/forjar-697-v1-35-0-lookahead (head d6e7e542)

Surveyed fresh on 2026-10-27 by fetching each vendor page named below. Nothing here
is from memory. Where a page did not say something, that is recorded as "not
documented" and no behaviour is assumed. Pages that failed to load (Puppet type
reference, which redirected to an index; the Nixpkgs dockerTools section, which fell
outside the fetched text; Chef group resource) are NOT cited as sources.

## Behaviour 1: supplementary-group convergence (src/resources/user.rs)

- Ansible `user` module, https://docs.ansible.com/ansible/latest/collections/ansible/builtin/user_module.html
  `append: false` means the user "will only be added to the groups specified in
  `groups`, removing them from all other groups". `group` sets the primary group as a
  separate parameter, and an empty `groups` removes the user "from all groups except
  the primary group". The page does not say what happens when the primary group is
  also listed in `groups`.
- Salt `user` state, https://docs.saltproject.io/en/latest/ref/states/all/salt.states.user.html
  `groups` is "a list of groups to assign the user to"; `remove_groups` defaults to
  True and removes memberships not named; an empty list removes the user "from all
  groups except the default group"; unset means "no changes". The primary group is
  set by `gid` and is outside the `groups` logic. Whether the primary may appear in
  `groups` is not documented.
- Chef `user` resource, https://docs.chef.io/resources/user/ documents only the
  primary group (`gid`) and says nothing about secondary groups, so Chef gives no
  evidence either way for this behaviour.
- Finding: both tools that document it treat the primary group as separate from
  supplementary membership and compare the declared supplementary set against the
  live one. forjar's user check does the same and additionally drops a declared
  primary from `groups:` before comparing (src/resources/user.rs:60). That extra
  tolerance goes beyond what either vendor documents; it is a forjar choice,
  recorded here as such.

## Behaviour 2: completion checks that ask the box

- Chef guards, https://docs.chef.io/resources/user/ (guard properties): `only_if`
  "Allow a resource to execute only if the condition returns `true`"; `not_if`
  "Prevent a resource from executing when the condition returns `true`"; a string
  guard "is executed as a shell command. If the command returns `0`, the guard is applied".
- Salt requisites, https://docs.saltproject.io/en/latest/ref/states/requisites.html
  under "Altering States", `unless` "specifies that a state should only run when any of
  the specified commands return `False`" and `onlyif` "specifies that if each command
  listed in `onlyif` returns `True`, then the state is run".
- Ansible `docker_image`, https://docs.ansible.com/ansible/latest/collections/community/docker/docker_image_module.html
  asks the daemon whether an image exists by name and tag before acting.
- Finding: asking the host with a shell probe, not trusting a stored record, is the
  established pattern in Chef and Salt. forjar's `completion_check` seeding in
  src/core/executor/lockless_check.rs is the same pattern.

## Behaviour 3: the -r/-g selection

- Terraform plan, https://developer.hashicorp.com/terraform/cli/commands/plan
  `-target` limits planning to the matching resource and Terraform will "extend the
  selection to include all other objects that those selections depend on either
  directly or indirectly". The same page warns against routine use because it "can
  lead to undetected configuration drift".
- Ansible and Salt selection flags were not fetched and are not claimed.
- Finding: closing a selection over dependencies, so plan and apply agree on one
  set, matches Terraform (src/cli/plan_selector.rs:131, src/cli/apply_selection/closure.rs:109).
  The drift warning supports this branch's rule that a filtered-out resource is
  neither checked on the host nor recorded (src/core/executor/lockless_check.rs:95).

## Behaviour 4: docker image pull

- Ansible `community.docker.docker_image`, URL above: with `source: pull` the image is
  pulled only when missing unless `force_source` is set; `pull.platform` is not used
  to decide whether to pull; pulling by digest is not documented (image IDs are
  documented only for absent, load and local).
- Finding: the surveyed tool leaves digest pinning undocumented. forjar's
  `state: pulled` requires a well-formed `@sha256:` digest at validation and verifies
  it after the pull (src/resources/docker.rs:11), which is stricter than the tool surveyed.

## Behaviour 5: arch-neutral CI

Not re-surveyed. The cache-key and runs-on survey already committed for this ticket is
in .quorum/ci-arch-neutral-lint.json (Swatinem/rust-cache, actions/cache, sccache).

## Verdict

No crux row is owed. On the behaviours where documentation exists, the branch matches
the surveyed tools: primary group kept apart from supplementary groups (Ansible, Salt),
shell-probe guards that ask the box (Chef, Salt), dependency-closed selection (Terraform).
Two places go beyond the documentation and are forjar's own choices: tolerating a primary
group listed in `groups:`, and requiring a sha256 digest for docker pulls.
