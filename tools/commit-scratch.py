"""Commits a scratch copy's changes onto the shared repo's current HEAD by a three-way merge.

    python -I commit_scratch.py <scratch dir> <base rev the scratch was made from> <message file>

Every file under crates, docs, tools and assets that differs from the base is taken from the
scratch copy. The merge base is the scratch's base, so commits another session landed since are
kept. A conflict in a translation table or the briefs is merged by key (the scratch wins on keys
it changed); any other conflict stops the commit. Then the index of the touched paths is synced,
and the commit is pushed. The copy is then refreshed to the new commit, which is the base to
give next time. Run from the repo root.
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

scratch, base, message_file = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
env = dict(os.environ)


def git(*args, input=None, index=None, check=True):
    e = dict(env)
    if index:
        e['GIT_INDEX_FILE'] = index
    out = subprocess.run(['git', *args], input=input, capture_output=True, env=e)
    if check and out.returncode != 0:
        raise SystemExit(f"git {' '.join(args)}: {out.stderr.decode('utf-8', 'replace')}")
    return out


def blob(rev, path):
    out = git('show', f'{rev}:{path}', check=False)
    return out.stdout.decode('utf-8') if out.returncode == 0 else None


head = git('rev-parse', 'HEAD').stdout.decode().strip()
base = git('rev-parse', base).stdout.decode().strip()
index = str(Path(tempfile.gettempdir()) / 'commit-scratch.index')
if os.path.exists(index):
    os.remove(index)
git('read-tree', base, index=index)
changed = []
for top in ('crates', 'docs', 'tools', 'assets', '.claude'):
    for path in sorted((scratch / top).rglob('*')):
        if not path.is_file():
            continue
        rel = path.relative_to(scratch).as_posix()
        sha = git('hash-object', '-w', str(path)).stdout.decode().strip()
        old = git('rev-parse', '-q', '--verify', f'{base}:{rel}', check=False).stdout.decode().strip()
        if sha != old:
            git('update-index', '--add', '--cacheinfo', f'100644,{sha},{rel}', index=index)
            changed.append(rel)
if not changed:
    raise SystemExit('nothing changed')
mine_tree = git('write-tree', index=index).stdout.decode().strip()
mine = git('commit-tree', mine_tree, '-p', base, input=b'scratch').stdout.decode().strip()

merged = git('merge-tree', '--write-tree', f'--merge-base={base}', head, mine, check=False)
lines = merged.stdout.decode('utf-8').splitlines()
tree = lines[0]
conflicted = sorted({line.split('\t')[1] for line in lines[1:] if '\t' in line and line.split()[2] in ('1', '2', '3')})
if conflicted:
    git('read-tree', tree, index=index)
    for path in conflicted:
        if not (path.startswith('docs/i18n/') and path.endswith('.json')):
            raise SystemExit(f'conflict in {path}: merge it by hand')
        raw = blob(head, path)
        theirs, old, ours = json.loads(raw), json.loads(blob(base, path) or 'null'), json.loads(blob(mine, path))
        if isinstance(ours, dict):
            for key, value in ours.items():
                if (old or {}).get(key) != value:
                    theirs[key] = value
            ind = len(raw.split('\n')[1]) - len(raw.split('\n')[1].lstrip(' '))
            text = json.dumps(theirs, indent=ind, ensure_ascii=False) + ('\n' if raw.endswith('\n') else '')
        else:
            before = {e['key']: e for e in (old or [])}
            mine_changed = {e['key']: e for e in ours if before.get(e['key']) != e}
            theirs = [mine_changed.pop(e['key'], e) for e in theirs] + list(mine_changed.values())
            text = json.dumps(theirs, indent=2 if raw.startswith('[\n  ') else 1, ensure_ascii=False) + '\n'
        sha = git('hash-object', '-w', '--stdin', input=text.encode('utf-8')).stdout.decode().strip()
        git('update-index', '--cacheinfo', f'100644,{sha},{path}', index=index)
    tree = git('write-tree', index=index).stdout.decode().strip()
message = message_file.read_bytes()
commit = git('commit-tree', tree, '-p', head, input=message).stdout.decode().strip()
git('update-ref', 'refs/heads/main', commit, head)
git('reset', '-q', 'HEAD', '--', *changed)
push = git('push', '-q', 'origin', 'main', check=False)
# The copy now holds the new commit, other sessions' work included, so its next base is that
# commit: a copy left as it was would hand an older file back on the next commit.
archive = subprocess.run(['git', 'archive', commit], capture_output=True, check=True).stdout
subprocess.run(['tar', '-x', '-C', str(scratch)], input=archive, check=True)
print(commit[:7], f'{len(changed)} files; the copy now holds {commit[:7]}, the base for the next commit;', 'merged by key: ' + ', '.join(conflicted) if conflicted else '', 'pushed' if push.returncode == 0 else 'PUSH FAILED: ' + push.stderr.decode())
