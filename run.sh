source .env
mkdir -p target/$PROFILE/config
yq eval-all 'select(fileIndex==0) * select(fileIndex==1) * select(fileIndex==2)' \
    config/base.yaml config/$FRIDA_ENV.yaml $1/config/$FRIDA_ENV.yaml > target/$PROFILE/config/total_config.yaml
cargo run -p $1 --bin $2