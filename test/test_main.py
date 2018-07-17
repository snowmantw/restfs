import restfs_lib
import subprocess
import argparse
from pathlib import Path
from sys import argv
from os import mkdir
from multiprocessing import Process
from time import sleep

def do_mount(path):

  if not path.exists():
    mkdir(path)

  try:
    # Block call.
    restfs_lib.mount(str(path))
  except KeyboardInterrupt:
    pass

def handle_args():
  parser = argparse.ArgumentParser()
  parser.add_argument('--root', dest='root', default='/tmp',
          help='where to put mounted directory (default: "/tmp")')
  parser.add_argument('--protocol', dest='protocol', default='https',
          help='http or https (default: "https")')
  parser.add_argument('hostname', help='which host should be mounted (ex: www.google.com)')
  return parser.parse_args()

try:
  args = handle_args()
  path = Path(args.root, args.hostname)
  print(path)
  p = Process(target=do_mount, args=(path,))
  p.start()
  p.join()
except KeyboardInterrupt:
  subprocess.run(['umount', str(path)])

