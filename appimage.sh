set -e

export APPIMAGE_EXTRACT_AND_RUN=1
apt-get update
apt-get install -y libgtk-3-dev patchelf
wget https://github.com/TheAssassin/appimagecraft/releases/download/continuous/appimagecraft-x86_64.AppImage
chmod +x appimagecraft-x86_64.AppImage
./appimagecraft-x86_64.AppImage
