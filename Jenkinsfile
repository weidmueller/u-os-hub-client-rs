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
            'u-os-hub-client-rs: Build for x86', 
            'u-os-hub-client-rs: Build for ARMv7', 
            'u-os-hub-client-rs: Build for ARM64', 
            'u-os-hub-client-rs: Unit tests',
            'u-os-hub-client-rs: Run linter',
            'u-os-hub-client-rs: Gen Docs',
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
        stage('u-os-hub-client-rs: Build for x86') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh 'cargo build --all-targets'
                }
            }
        }     
        stage('u-os-hub-client-rs: Build for ARMv7') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh './tools/build-for-target.sh dev armv7-unknown-linux-gnueabihf'
                }
            }
        }  
        stage('u-os-hub-client-rs: Build for ARM64') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh './tools/build-for-target.sh dev aarch64-unknown-linux-gnu'
                }
            }
        }     
        stage('u-os-hub-client-rs: Unit tests') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh 'cargo test'
                }
            }
        }
        stage('u-os-hub-client-rs: Run linter') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh 'cargo clippy -- -D warnings'
                }
            }
        }
        stage('u-os-hub-client-rs: Gen Docs') { 
            steps {
                gitlabCommitStatus(name:"$STAGE_NAME") {
                    sh 'cargo doc --no-deps'
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
