# GPG test fixtures

A throwaway key pair for `file_copy_decrypts_with_gpg` in `app/tests/apply.rs`. It is public on
purpose and protects nothing: never encrypt real secrets to it.

- `secret-key.asc`: RSA 3072 key, no passphrase
  (`comtrya test key (public, for tests only) <test@comtrya.dev>`)
- `credentials.fish.asc`: `set -gx API_TOKEN not-a-real-token`, encrypted to that key

It's RSA because GnuPG 2.4.8 with libgcrypt 1.12 (Ubuntu 26.04) can't decrypt with an
imported cv25519 secret key ("Bad secret key"). The RSA key works with GnuPG 2.4.4, 2.4.8 and 2.5.

Recreate both with a temporary `GNUPGHOME`:

```sh
export GNUPGHOME=$(mktemp -d)
gpg --batch --passphrase '' --quick-gen-key 'comtrya test key (public, for tests only) <test@comtrya.dev>' rsa3072 sign,encr never
fpr=$(gpg --list-keys --with-colons | awk -F: '/^fpr/ {print $10; exit}')
gpg --armor --export-secret-keys "$fpr" > secret-key.asc
printf 'set -gx API_TOKEN not-a-real-token\n' | gpg --armor --encrypt --recipient "$fpr" --trust-model always > credentials.fish.asc
```
