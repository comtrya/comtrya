# GPG test fixtures

A throwaway key pair for `file_copy_decrypts_with_gpg` in `app/tests/apply.rs`. It is public on
purpose and protects nothing: never encrypt real secrets to it.

- `secret-key.asc`: ed25519 primary key with a cv25519 encryption subkey, no passphrase
  (`comtrya test key (public, for tests only) <test@comtrya.dev>`)
- `credentials.fish.asc`: `set -gx API_TOKEN not-a-real-token`, encrypted to that key

Recreate both with a temporary `GNUPGHOME`:

```sh
export GNUPGHOME=$(mktemp -d)
gpg --batch --passphrase '' --quick-gen-key 'comtrya test key (public, for tests only) <test@comtrya.dev>' ed25519 sign never
fpr=$(gpg --list-keys --with-colons | awk -F: '/^fpr/ {print $10; exit}')
gpg --batch --passphrase '' --quick-add-key "$fpr" cv25519 encr never
gpg --armor --export-secret-keys "$fpr" > secret-key.asc
printf 'set -gx API_TOKEN not-a-real-token\n' | gpg --armor --encrypt --recipient "$fpr" --trust-model always > credentials.fish.asc
```
