# Dependencies

In comtrya, manifests can depend on other manifests. This is helpful when you need to ensure a set of actions run before another because they depend on them in some way. This is done by specifying what manifests a manifest depends on.

Suppose we have a manifest for creating users and another manifest for creating groups. We may want to ensure that the groups are created before adding users to a group. In this examples, we have two files; `users.yaml` and `groups.yaml`. To ensure that groups.yaml runs before users.yaml, we write our groups.yaml as we normally would.

## groups.yaml
```yaml
actions:
  - action: group.add
    group_name: testgroup
```

## users.yaml
```yaml
depends:
  - groups

actions:
  - action: user.add
    fullename: testuser
	home_dir: /home/test
	username: test
	shell: sh
	group:
	  - testgroup
```

As shown, at the top of the `users.yaml` file, `depends` takes a lists of manifests that this manifest depends on.

## When something fails

If an action fails, comtrya skips the remaining actions of that manifest and every manifest that depends on it, directly or indirectly. All other manifests still run, so one run shows every problem at once. At the end, comtrya exits with a non-zero status, so scripts and CI notice the failure.

In the example above, if creating the group fails, `users.yaml` is skipped, while unrelated manifests are applied as usual.

A manifest that can't be loaded counts as failed in the same way: invalid YAML or TOML, an unknown action or field value, a template that can't be rendered, or a newer manifest `version`. So does a manifest whose `depends` names a manifest that doesn't exist. This also applies to `--dry-run` and `comtrya status`, so they can check manifests in CI.

A manifest whose `where` condition is false is not an error, even if its templates only work where it applies.
