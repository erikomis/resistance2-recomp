#!/usr/bin/env python3
"""Fetch the keys that decrypt the game's executables, from RPCS3's public source.

    tools/fetch_keys.py                 # write the key file if it is not there yet
    tools/fetch_keys.py --npdrm         # the same, and it must hold the 01.60 update's keys
    tools/fetch_keys.py --force         # download and write it again
    tools/fetch_keys.py --source FILE   # use a key_vault.cpp you already have
    tools/fetch_keys.py --source-h FILE # use a key_vault.h you already have

No key is in this repository. The disc's EBOOT.BIN is a SELF encrypted with
the PS3's application keys ("appldr", program type APP), and
those keys are published in RPCS3's source code, rpcs3/Crypto/key_vault.cpp.
This downloads that file from GitHub at a pinned commit, checks its SHA-256
(so the file is exactly the one this was tested with, whatever the network or
TLS in between), takes the APP entries (key revision, erk, riv) out of
KeyVault::LoadSelfAPPKeys, and writes them in scetool's key-file format for
tools/decrypt_self.py.

The 01.60 update's EBOOT.BIN (online play, tools/build_online.sh) is an NPDRM
SELF with a free licence (type 3): it needs the NPDRM entries of the same
file (KeyVault::LoadSelfNPDRMKeys) and two fixed keys, NP_KLIC_FREE and
NP_KLIC_KEY, which RPCS3 keeps in rpcs3/Crypto/key_vault.h. That file is
downloaded from the same commit and checked by its own SHA-256. Without
--npdrm a key_vault.h that cannot be had is not an error (the disc's
executable does not need it); with --npdrm it is, and a key file without
those keys is downloaded again.

The key file goes to a per-user folder outside the repository, readable only
by you (mode 600):

    macOS  ~/Library/Application Support/resistance-recomp-2/keys/self_keys.txt
    other  $XDG_DATA_HOME (~/.local/share)/resistance-recomp-2/keys/self_keys.txt

--out or RES_SELF_KEYS put it elsewhere. The path is printed on stdout (and
nothing else); progress and errors go to stderr. No key is ever printed.

Only the Python standard library is used, so this runs before the toolkit's
Python packages are installed.
"""
import argparse
import hashlib
import os
import re
import ssl
import sys
import urllib.error
import urllib.request

# The last RPCS3 commit that changed key_vault.cpp when this was written
# ("Use std::string_view in crypto code", 2026-06-16), and the SHA-256 of the
# files at that commit. A hash is not a key: it only says which file is right.
RPCS3_COMMIT = 'b8ca6d953f734900c23f2cfbe79a36e598171553'
KEY_VAULT_SHA256 = 'a2947ff6b681258c198552ec6344e16685e2df649018206daa3f41a0a1aadbd7'
KEY_VAULT_H_SHA256 = 'd5d0cede74c7900bb23ca8d60066194648ed4b976eaee60315c0bcbed7dc2aaf'
_RAW = 'https://raw.githubusercontent.com/RPCS3/rpcs3/%s/rpcs3/Crypto/' % RPCS3_COMMIT
KEY_VAULT_URL = _RAW + 'key_vault.cpp'
KEY_VAULT_H_URL = _RAW + 'key_vault.h'

# The key revisions to expect: 0x0001 for the Resistance 2 disc's EBOOT.BIN
# (and for Resistance: Fall of Man's 01.00 executables), 0x0002 for that
# game's v1.7 update. Every APP entry is written; these must be among them.
NEEDED_REVISIONS = (0x0001, 0x0002)
# NPDRM: 0x0001 for the Resistance 2 01.60 update's EBOOT.BIN. Every NPDRM
# entry is written; this one must be among them.
NEEDED_NPDRM_REVISIONS = (0x0001,)
# The fixed NPDRM keys from key_vault.h: our name (decrypt_self.py's) -> RPCS3's.
KLIC_KEYS = (('NP_klic_free', 'NP_KLIC_FREE'), ('NP_klic_key', 'NP_KLIC_KEY'))


def default_path():
    if os.environ.get('RES_SELF_KEYS'):
        return os.environ['RES_SELF_KEYS']
    home = os.path.expanduser('~')
    if sys.platform == 'darwin':
        base = os.path.join(home, 'Library', 'Application Support')
    else:
        base = os.environ.get('XDG_DATA_HOME') or os.path.join(home, '.local', 'share')
    return os.path.join(base, 'resistance-recomp-2', 'keys', 'self_keys.txt')


def log(msg):
    print(msg, file=sys.stderr)


class FetchError(Exception):
    pass


def download(url, timeout):
    req = urllib.request.Request(url, headers={'User-Agent': 'resistance2-recomp fetch_keys.py'})

    def get(ctx):
        with urllib.request.urlopen(req, timeout=timeout, context=ctx) as r:
            return r.read()
    try:
        try:
            return get(ssl.create_default_context())
        except urllib.error.URLError as e:
            if not isinstance(e.reason, ssl.SSLCertVerificationError):
                raise
            # A python.org Python on macOS has no CA certificates until its
            # "Install Certificates.command" is run. The file is checked
            # against its pinned SHA-256 below, so a connection that cannot
            # be verified still cannot hand us a different file.
            log('  (this Python cannot verify GitHub\'s certificate; the file is checked by its SHA-256 instead)')
            return get(ssl._create_unverified_context())
    except (urllib.error.URLError, OSError) as e:
        reason = getattr(e, 'reason', e)
        raise FetchError(
            'could not download the decryption keys from GitHub (%s).\n'
            'Check that this computer is connected to the internet and try again.\n'
            'They come from: %s' % (reason, url))


def fetch(path, url, sha256, timeout):
    """(bytes, where they came from) of a pinned RPCS3 file, checked by its SHA-256."""
    if path:
        with open(path, 'rb') as f:
            raw = f.read()
        source = os.path.abspath(path)
    else:
        log('downloading the decryption keys from RPCS3\'s source code on GitHub (%s)...'
            % url.rsplit('/', 1)[-1])
        raw = download(url, timeout)
        source = url
    got = hashlib.sha256(raw).hexdigest()
    if got != sha256:
        raise FetchError('%s is not the expected file (SHA-256 %s, expected %s); nothing was written'
                         % (url.rsplit('/', 1)[-1], got, sha256))
    return raw, source


def parse_self_keys(text, loader, kind, needed):
    """[(revision, erk hex, riv hex)] from KeyVault::<loader>, entries of key
    type <kind>, in file order (RPCS3 takes the first entry of a revision; so
    does decrypt_self.py)."""
    m = re.search(r'void\s+KeyVault::%s\s*\(\s*\)\s*\{(.*?)\n\}' % loader, text, re.S)
    if not m:
        raise FetchError('key_vault.cpp has no KeyVault::%s' % loader)
    entry = re.compile(
        r'sk_\w+_arr\.emplace_back\(\s*0x[0-9A-Fa-f]+\s*,\s*0x[0-9A-Fa-f]+\s*,\s*0x([0-9A-Fa-f]+)\s*,'
        r'\s*%s\s*,\s*"([0-9A-Fa-f]*)"\s*,\s*"([0-9A-Fa-f]*)"' % kind)
    keys, seen = [], set()
    for rev, erk, riv in entry.findall(m.group(1)):
        rev = int(rev, 16)
        if len(erk) != 64 or len(riv) != 32 or rev in seen:
            continue
        seen.add(rev)
        keys.append((rev, erk.upper(), riv.upper()))
    missing = [r for r in needed if r not in seen]
    if missing:
        raise FetchError('key_vault.cpp has no %s key for revision %s'
                         % (kind, ', '.join('0x%04X' % r for r in missing)))
    return keys


def parse_app_keys(text):
    return parse_self_keys(text, 'LoadSelfAPPKeys', 'KEY_APP', NEEDED_REVISIONS)


def parse_npdrm_keys(text):
    return parse_self_keys(text, 'LoadSelfNPDRMKeys', 'KEY_NPDRM', NEEDED_NPDRM_REVISIONS)


def parse_klic(text):
    """{our name: hex} of NP_KLIC_FREE and NP_KLIC_KEY in key_vault.h."""
    out = {}
    for ours, theirs in KLIC_KEYS:
        m = re.search(r'constexpr\s+u8\s+%s\s*\[\s*0x10\s*\]\s*=\s*\{([^}]*)\}' % theirs, text)
        if not m:
            raise FetchError('key_vault.h has no %s' % theirs)
        b = [int(x, 16) for x in re.findall(r'0x([0-9A-Fa-f]{1,2})', m.group(1))]
        if len(b) != 16:
            raise FetchError('key_vault.h: %s is %d bytes, not 16' % (theirs, len(b)))
        out[ours] = bytes(b).hex().upper()
    return out


def key_file_text(keys, source, npdrm=(), klic=None):
    out = ['# SELF keys for program types APP (appldr) and NPDRM, in scetool\'s',
           '# key-file format, for tools/decrypt_self.py. Written by',
           '# tools/fetch_keys.py from %s' % source,
           '# Do not share this file or put it in a repository.', '']
    for rev, erk, riv in keys:
        out += ['[appldr_%04X]' % rev, 'type=SELF', 'revision=%04X' % rev, 'self_type=APP',
                'erk=%s' % erk, 'riv=%s' % riv, '']
    for rev, erk, riv in npdrm:
        out += ['[npdrm_%04X]' % rev, 'type=SELF', 'revision=%04X' % rev, 'self_type=NPDRM',
                'erk=%s' % erk, 'riv=%s' % riv, '']
    for name, _ in KLIC_KEYS:
        if klic and name in klic:
            out += ['[%s]' % name, 'key=%s' % klic[name], '']
    return '\n'.join(out)


def write_private(path, text):
    d = os.path.dirname(path)
    if d:
        os.makedirs(d, mode=0o700, exist_ok=True)
    tmp = path + '.tmp'
    try:
        os.unlink(tmp)
    except FileNotFoundError:
        pass
    # Created 600 from the start, so the keys are never readable by others,
    # not even for a moment.
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, 'w', encoding='ascii') as f:
        f.write(text)
    os.chmod(tmp, 0o600)
    os.replace(tmp, path)


def usable(path, npdrm=False):
    """True when the key file is there and has the APP keys the game needs
    (and with npdrm, the 01.60 update's NPDRM key and the two klic keys)."""
    try:
        with open(path, encoding='ascii', errors='replace') as f:
            text = f.read()
    except OSError:
        return False
    blocks = re.findall(r'^\[[^\]]+\]\s*\n((?:[^\[\n][^\n]*\n?)*)', text, re.M)
    revs = {'APP': set(), 'NPDRM': set()}
    for b in blocks:
        t = re.search(r'^self_type=(\w+)\s*$', b, re.M)
        r = re.search(r'^revision=([0-9A-Fa-f]+)\s*$', b, re.M)
        if t and r and t.group(1).upper() in revs:
            revs[t.group(1).upper()].add(int(r.group(1), 16))
    if not all(r in revs['APP'] for r in NEEDED_REVISIONS):
        return False
    if npdrm:
        if not all(r in revs['NPDRM'] for r in NEEDED_NPDRM_REVISIONS):
            return False
        for name, _ in KLIC_KEYS:
            if not re.search(r'^\[%s\]\s*\nkey=[0-9A-Fa-f]{32}\s*$' % name, text, re.M):
                return False
    return True


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--out', default=default_path(), help='key file to write (default: %(default)s)')
    ap.add_argument('--force', action='store_true', help='download and write it even if it is there')
    ap.add_argument('--npdrm', action='store_true',
                    help='the key file must also hold the NPDRM keys (the 01.60 update)')
    ap.add_argument('--source', help='a key_vault.cpp already on disk (still checked by its SHA-256)')
    ap.add_argument('--source-h', help='a key_vault.h already on disk (still checked by its SHA-256)')
    ap.add_argument('--timeout', type=float, default=60, help='download timeout, seconds (default 60)')
    a = ap.parse_args(argv)
    path = os.path.abspath(os.path.expanduser(a.out))

    if not a.force and usable(path, a.npdrm):
        print(path)
        return 0
    try:
        raw, source = fetch(a.source, KEY_VAULT_URL, KEY_VAULT_SHA256, a.timeout)
        text = raw.decode('utf-8', 'replace')
        keys = parse_app_keys(text)
        npdrm, klic = (), None
        try:
            raw_h, _ = fetch(a.source_h, KEY_VAULT_H_URL, KEY_VAULT_H_SHA256, a.timeout)
            klic = parse_klic(raw_h.decode('utf-8', 'replace'))
            npdrm = parse_npdrm_keys(text)
        except (FetchError, OSError) as e:
            if a.npdrm:
                raise
            log('  (no NPDRM keys: %s; the disc\'s executable does not need them)' % e)
        write_private(path, key_file_text(keys, source, npdrm, klic))
    except (FetchError, OSError) as e:
        log('error: %s' % e)
        return 1
    log('  %d key sets written to %s%s' % (len(keys) + len(npdrm), path,
                                          ' (with the NPDRM keys)' if klic else ''))
    print(path)
    return 0


if __name__ == '__main__':
    sys.exit(main())
