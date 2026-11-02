from package.sub.module import run

CHANNEL_READY = False

if CHANNEL_READY:
    pass
elif __name__ == "__channelexec__":
    run()
