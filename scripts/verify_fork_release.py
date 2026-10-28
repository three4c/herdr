#!/usr/bin/env python3
"""Verify public Aira assets, installation, and update using only temporary paths."""
import argparse
import hashlib
import json
import os
import platform
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

try:
    from .fork_release import REPOSITORY, TARGETS, release_version, version_parts
except ImportError:
    from fork_release import REPOSITORY, TARGETS, release_version, version_parts
BASE_URL = f'https://github.com/{REPOSITORY}/releases'

def run(args, env=None, timeout=180):
    result = subprocess.run([str(x) for x in args], env=env, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f'{args}: {result.stdout}\n{result.stderr}')
    return result.stdout

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def verify_assets(assets, tag, commit):
    number = release_version(tag)
    expected = {f'herdr-{target}' for target in TARGETS} | {'install.sh', 'fork-latest.json', 'LICENSE', 'FORK-NOTICE.txt', 'THIRD-PARTY-NOTICES.txt', 'SHA256SUMS'}
    sums = {}
    for line in (assets / 'SHA256SUMS').read_text(encoding='utf-8').splitlines():
        sha, name = line.split('  ')
        if not (Path(name).name == name and digest(assets / name) == sha):
            raise ValueError("public release verification failed")
        sums[name] = sha
    if not (set(sums) == expected - {'SHA256SUMS'}):
        raise ValueError("public release verification failed")
    manifest = json.loads((assets / 'fork-latest.json').read_text(encoding='utf-8'))
    if not (manifest['distribution'] == REPOSITORY and manifest['release'] == number):
        raise ValueError("public release verification failed")
    if not (manifest['commit'] == commit and manifest['version'].endswith(f'-aira.{number}')):
        raise ValueError("public release verification failed")
    for target in TARGETS:
        data = (assets / f'herdr-{target}').read_bytes()
        if not (manifest['assets'][target] == f'{BASE_URL}/download/{tag}/herdr-{target}'):
            raise ValueError("public release verification failed")
        if not (manifest['sha256'][target] == hashlib.sha256(data).hexdigest()):
            raise ValueError("public release verification failed")
        if target.startswith('linux-'):
            if not (data[:6] == b'\x7fELF\x02\x01'):
                raise ValueError("public release verification failed")
            if not (struct.unpack_from('<H', data, 18)[0] == (62 if target.endswith('x86_64') else 183)):
                raise ValueError("public release verification failed")
        else:
            if not (struct.unpack_from('<II', data) == (0xfeedfacf, 0x1000007 if target.endswith('x86_64') else 0x100000c)):
                raise ValueError("public release verification failed")
    if manifest['version'] != f"{manifest['base_version']}-aira.{number}":
        raise ValueError('manifest base version and Aira identity disagree')
    if manifest['protocol'] <= 0 or manifest['endpoint_generation'] <= 0:
        raise ValueError('manifest protocol metadata is invalid')
    return manifest, sums

def main(tag, commit, previous_tag=None, report_path=None):
    number = release_version(tag)
    metadata = json.loads(run(['gh', 'release', 'view', tag, '--repo', REPOSITORY, '--json', 'tagName,isDraft,isPrerelease,assets,url']))
    if not (metadata['tagName'] == tag and not metadata['isDraft'] and not metadata['isPrerelease']):
        raise ValueError("public release verification failed")
    expected = {f'herdr-{target}' for target in TARGETS} | {'install.sh', 'fork-latest.json', 'LICENSE', 'FORK-NOTICE.txt', 'THIRD-PARTY-NOTICES.txt', 'SHA256SUMS'}
    if not ({asset['name'] for asset in metadata['assets']} == expected):
        raise ValueError("public release verification failed")
    with tempfile.TemporaryDirectory(prefix='aira-public-verification-') as tmp:
        root = Path(tmp)
        assets = root / 'assets'
        assets.mkdir()
        run(['gh', 'release', 'download', tag, '--repo', REPOSITORY, '--dir', assets], timeout=300)
        manifest, sums = verify_assets(assets, tag, commit)
        prefix = root / 'install'
        prefix.mkdir()
        upstream = prefix / 'herdr'
        sentinel = b'#!/bin/sh\nexit 93\n'
        upstream.write_bytes(sentinel)
        upstream.chmod(0o700)
        trace = root / 'trace'
        trace.mkdir()
        requests = root / 'requests.jsonl'
        curl = shutil.which('curl')
        if not (curl):
            raise ValueError("public release verification failed")
        (trace / 'curl').write_text(f'#!{sys.executable}\n' + '''import json,os,sys
from pathlib import Path
args=sys.argv[1:]
for arg in args:
 if arg.startswith(('https://','http://')):
  with open(os.environ['AIRA_REQUESTS'],'a',encoding='utf-8') as log:log.write(json.dumps(arg)+'\\n')
  if not arg.startswith('https://github.com/three4c/herdr/releases/'):sys.exit(97)
os.execv(os.environ['AIRA_REAL_CURL'],[os.environ['AIRA_REAL_CURL'],*args])
''', encoding='utf-8')
        (trace / 'curl').chmod(0o700)
        config = root / 'config.toml'
        config.write_text('[update]\nchannel = "preview"\n', encoding='utf-8')
        config_bytes = config.read_bytes()
        env = {k:v for k,v in os.environ.items() if not k.startswith('HERDR_') and k not in ('GH_TOKEN','GITHUB_TOKEN')}
        env.update(PATH=f'{trace}:{prefix}:'+os.environ['PATH'], HERDR_INSTALL_DIR=str(prefix), HERDR_CONFIG_PATH=str(config), HERDR_SOCKET_PATH=str(root/'absent.sock'), HERDR_CLIENT_SOCKET_PATH=str(root/'absent-client.sock'), XDG_CONFIG_HOME=str(root/'config'), XDG_STATE_HOME=str(root/'state'), XDG_RUNTIME_DIR=str(root/'runtime'), AIRA_REQUESTS=str(requests), AIRA_REAL_CURL=curl)
        installer = root / 'install.sh'
        run(['curl','-fsSL','--retry','3',f'{BASE_URL}/latest/download/install.sh','-o',installer], env)
        if not (digest(installer) == sums['install.sh']):
            raise ValueError("public release verification failed")
        run(['/bin/sh', installer], env)
        binary = prefix / 'herdr-aira'
        expected_version = f"herdr {manifest['version']}"
        if not (run([binary,'--version'],env).strip() == expected_version):
            raise ValueError("public release verification failed")
        expected_licenses = '\n'.join((assets / name).read_text(encoding='utf-8') for name in ('LICENSE','FORK-NOTICE.txt','THIRD-PARTY-NOTICES.txt'))
        if not (run([binary,'--licenses'],env) == expected_licenses):
            raise ValueError("public release verification failed")
        if not ('aira' in run([binary,'channel','show'],env)):
            raise ValueError("public release verification failed")
        run([binary,'update'],env)
        for channel in ('stable', 'preview'):
            result = subprocess.run([str(binary),'channel','set',channel],env=env,capture_output=True,text=True,timeout=30)
            if result.returncode == 0:
                raise ValueError('fork unexpectedly allowed upstream channel switching')
        old_version = None
        if previous_tag:
            previous = release_version(previous_tag)
            if version_parts(previous) >= version_parts(number):
                raise ValueError('previous release must precede this release')
            old_assets = root / 'previous'
            old_assets.mkdir()
            machine = 'aarch64' if platform.machine() in ('arm64','aarch64') else 'x86_64'
            host = ('macos' if platform.system() == 'Darwin' else 'linux') + '-' + machine
            old_name = 'herdr-' + host
            run(['gh','release','download',previous_tag,'--repo',REPOSITORY,'--pattern',old_name,'--pattern','SHA256SUMS','--dir',old_assets])
            old = old_assets / old_name
            expected_old = next(line.split('  ')[0] for line in (old_assets/'SHA256SUMS').read_text().splitlines() if line.endswith('  '+old_name))
            if not (digest(old) == expected_old):
                raise ValueError("public release verification failed")
            old.chmod(0o700)
            old_version = run([old,'--version'],env).strip()
            if not (old_version.endswith('-aira.' + previous)):
                raise ValueError("public release verification failed")
            shutil.copy2(old, binary)
            run([binary,'update'],env)
            if not (run([binary,'--version'],env).strip() == expected_version):
                raise ValueError("public release verification failed")
        machine = 'aarch64' if platform.machine() in ('arm64','aarch64') else 'x86_64'
        host = ('macos' if platform.system() == 'Darwin' else 'linux') + '-' + machine
        if not (digest(binary) == manifest['sha256'][host]):
            raise ValueError("public release verification failed")
        run(['/bin/sh',installer],env)
        if not (digest(binary) == manifest['sha256'][host]):
            raise ValueError("public release verification failed")
        if not (upstream.read_bytes() == sentinel and config.read_bytes() == config_bytes):
            raise ValueError("public release verification failed")
        urls = [json.loads(line) for line in requests.read_text().splitlines()]
        if not (urls and all(url.startswith(BASE_URL + '/') for url in urls)):
            raise ValueError("public release verification failed")
        report = {'tag':tag, 'commit':commit, 'version':expected_version, 'assets':sorted(expected), 'sha256':{name:digest(assets/name) for name in sorted(expected)}, 'old_version':old_version, 'host':host, 'requests':urls, 'result':'passed', 'ordinary_installation_modified':False, 'servers_started':False}
        if report_path:
            report_path.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
        print(json.dumps({key:report[key] for key in ('tag','version','old_version','host','result','servers_started')},ensure_ascii=False))

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tag',required=True)
    parser.add_argument('--commit',required=True)
    parser.add_argument('--previous-tag')
    parser.add_argument('--report',type=Path)
    args=parser.parse_args()
    main(args.tag,args.commit,args.previous_tag,args.report)
