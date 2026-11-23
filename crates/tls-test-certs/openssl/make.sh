#!/bin/sh
# usage: make.sh <keys dir> <out dir> <source file>
#
# Makes the three cases ring cannot sign (docs/design/tls.md, item 6), with openssl, into <out
# dir>, beside the cases surf-tls-test-certs wrote there first:
#
#   sha1      a leaf signed sha1WithRSAEncryption by the RSA 2048 intermediate
#   md5       a leaf signed md5WithRSAEncryption by the same intermediate
#   rsa-1024  a leaf with the RSA 1024 key, signed by the same intermediate with SHA-256
#
# The 1024-bit key of rsa-1024 is the leaf's own and verifies no signature on the path: a
# validator must apply the weak-key rule to the leaf's own key, not only to keys that verify a
# signature.
#
# Each leaf is for localhost (subjectAltName dNSName localhost and iPAddress 127.0.0.1), valid
# from 2025-01-01 to 2125-01-01, with a fixed serial number and subject. The intermediate and the
# root are those of ok-rsa2048, its key keys/rsa2048-0.pk8. RSA PKCS#1 v1.5 signing is
# deterministic, so a run makes the same bytes. Per case it writes <case>/chain.pem (the leaf,
# then the intermediate), <case>/root.pem and <case>/leaf.pk8, and appends the case's line to
# INDEX; then it writes <source file>: the openssl version, the date and the sha256 of each file.
#
# Run by the lane slow only (task tls-certs-slow); the gate never runs openssl. Needs openssl 3.4
# or later (-not_before, -not_after).

set -eu

if [ "$#" -ne 3 ]; then
  echo "usage: make.sh <keys dir> <out dir> <source file>" >&2
  exit 2
fi
keys=$1
out=$2
source=$3
time=2026-11-22T00:00:00Z
from=$out/ok-rsa2048

if [ ! -f "$from/chain.pem" ] || [ ! -f "$out/INDEX" ]; then
  echo "make.sh: $from/chain.pem or $out/INDEX is missing: run surf-tls-test-certs --out $out first" >&2
  exit 1
fi

work=$out/.openssl
rm -rf "$work"
mkdir -p "$work"

awk '/-----BEGIN CERTIFICATE-----/ { n++ } n == 2' "$from/chain.pem" > "$work/intermediate.pem"
test -s "$work/intermediate.pem"

printf '%s\n' \
  '[leaf]' \
  'basicConstraints = critical, CA:FALSE' \
  'keyUsage = critical, digitalSignature' \
  'extendedKeyUsage = serverAuth' \
  'subjectAltName = DNS:localhost, IP:127.0.0.1' \
  'subjectKeyIdentifier = hash' \
  'authorityKeyIdentifier = keyid' > "$work/leaf.cnf"

# leaf <case> <key name> <digest> <serial> <check>
leaf() {
  name=$1
  key=$keys/$2.pk8
  mkdir -p "$out/$name"
  openssl pkey -inform DER -in "$key" -pubout -out "$work/$name.pub.pem"
  openssl x509 -new -force_pubkey "$work/$name.pub.pem" -subj /CN=localhost \
    -CA "$work/intermediate.pem" -CAkey "$keys/rsa2048-0.pk8" -CAkeyform DER \
    -set_serial "$4" -not_before 20250101000000Z -not_after 21250101000000Z \
    "-$3" -extfile "$work/leaf.cnf" -extensions leaf -out "$work/$name.pem"
  cat "$work/$name.pem" "$work/intermediate.pem" > "$out/$name/chain.pem"
  cp "$from/root.pem" "$out/$name/root.pem"
  cp "$key" "$out/$name/leaf.pk8"
  printf '%s localhost %s CertificateNotTrusted %s\n' "$name" "$time" "$5" >> "$out/INDEX"
}

leaf sha1 rsa2048-1 sha1 0x40000000000000000000000000000001 WeakAlgorithm
leaf md5 rsa2048-1 md5 0x40000000000000000000000000000002 WeakAlgorithm
leaf rsa-1024 rsa1024 sha256 0x40000000000000000000000000000003 WeakKey

rm -rf "$work"

{
  echo "The three cases of openssl/make.sh: sha1, md5 and rsa-1024 (docs/design/tls.md, item 6)."
  echo "Written by make.sh with the files; the slow lane (task tls-certs-slow WRITE=1) copies it here."
  echo
  echo "openssl version: $(openssl version)"
  echo "date: $(date -u +%Y-%m-%d)"
  echo
  echo "sha256 of each output (sha256sum, paths under certs/):"
  echo
  cd "$out"
  for name in sha1 md5 rsa-1024; do
    sha256sum "$name/chain.pem" "$name/root.pem" "$name/leaf.pk8"
  done
} > "$source"
