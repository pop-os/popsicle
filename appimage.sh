set -e

export APPIMAGE_EXTRACT_AND_RUN=1
git config --global --add safe.directory /github/workspace
apt-get update
apt-get install -y help2man libdbus-1-dev libxkbcommon-dev patchelf pkg-config
wget https://github.com/TheAssassin/appimagecraft/releases/download/continuous/appimagecraft-x86_64.AppImage
chmod +x appimagecraft-x86_64.AppImage
./appimagecraft-x86_64.AppImage
