set -e

rm -rf debug

cd http_server
cargo build --release
cd ../server 
cargo build --release 
cd ..

mkdir debug
mv http_server/target/release/mhs-bin debug/mhs-bin 
mv server/target/release/server debug/server 
cp -r http_server/web debug/web 

echo Finished
