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
