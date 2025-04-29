pipeline {
    agent {
        dockerfile {
            label 'docker'
            dir '.devcontainer'
            reuseNode true
            additionalBuildArgs  '\
                --build-arg=USER_UID=$(id -u) \
                --build-arg=USER_GID=$(id -g) \
                '
        }
    }
    options {
        gitlabBuilds(builds: [
            'u-os-hub-client-rs: Check Code Formatting', 
            'u-os-hub-client-rs: Build & Test with uOS rust version and oldest possible dependencies', 
            'u-os-hub-client-rs: Build & Test with latest rust version and dependencies', 
            'u-os-hub-client-rs: Check for vulnerabilities'
        ])
    }
    stages {
        // ----------------------------------------------- Project Build Stages -----------------------------------------------
        stage('u-os-hub-client-rs: Check Code Formatting') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh 'cargo fmt --check'
                }
            }
        }
        stage('u-os-hub-client-rs: Build & Test with uOS rust version and oldest possible dependencies') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh 'rm -f Cargo.lock'
                    sh 'cargo clean'
                    sh 'cargo +nightly -Zminimal-versions update'
                    sh './tools/build-for-target.sh dev x86_64-unknown-linux-gnu ${U_OS_RUST_VERSION}'
                    sh './tools/build-for-target.sh dev armv7-unknown-linux-gnueabihf ${U_OS_RUST_VERSION}'
                    sh './tools/build-for-target.sh dev aarch64-unknown-linux-gnu ${U_OS_RUST_VERSION}'
                    sh 'cargo +${U_OS_RUST_VERSION} clippy --all-features --all-targets -- -D warnings'
                    sh 'RUSTDOCFLAGS="-D warnings" cargo +${U_OS_RUST_VERSION} doc --no-deps'
                    sh 'cargo +${U_OS_RUST_VERSION} test --all-features --target x86_64-unknown-linux-gnu'
                }
            }
        } 
        stage('u-os-hub-client-rs: Build & Test with latest rust version and dependencies') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh 'rm -f Cargo.lock'
                    sh 'cargo clean'
                    sh './tools/build-for-target.sh dev x86_64-unknown-linux-gnu'
                    //high level code and lib crate must also build without LL api feature flag
                    sh 'cargo build'
                    sh 'cargo build --example u-os-hub-example-provider'
                    sh 'cargo build --example u-os-hub-example-consumer'
                    sh './tools/build-for-target.sh dev armv7-unknown-linux-gnueabihf'
                    sh './tools/build-for-target.sh dev aarch64-unknown-linux-gnu'
                    sh 'cargo clippy --all-features --all-targets -- -D warnings'
                    sh 'RUSTDOCFLAGS="-D warnings" cargo doc --no-deps'
                    sh 'cargo test --all-features --target x86_64-unknown-linux-gnu'
                }
            }
        }    
        stage('u-os-hub-client-rs: Check for vulnerabilities') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh 'cargo audit'
                }
            }
        }
    }
}
