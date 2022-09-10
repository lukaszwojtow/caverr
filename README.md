# Caverr
## A no-nonsense, simple as a shovel, (very) personal backup / encryption tool.

This is really just an RSA lib wrapped into CLI. Allows for encrypting and decrypting files and directories.

# Usage:
1. Generate keys:
    `caverr -c keys`
    Copy private key to a file, keep it safe. Copy public key to easily accessible file, this will be used for encryption.
2. Create target dir:
    `mkdir /storage/backup`
3. Encrypt home folder (-k = key file, -s = source, -t = target):

    `caverr -c enc -k ~/public.key -s ~ -t /storage/backup`

    It will only encrypt files that:
    - don't exist in /storage/backup, or
    - have later modification time
    (i.e. calling it was with the same args will encrypt only new files)
4. Optional: you can track progress by sending SIG_HUP to the process:
    `ps aux | grep caverr` to get pid
    `kill -1 <PID>` will print stats on the screen.
5. Decrypt file(s):
    `caverr -c dec -k /safe/private.key -s /storage/backup -t /home/recovered`
