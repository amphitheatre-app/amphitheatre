// Copyright 2022 The Amphitheatre Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations under
// the License.

/// Inspired by Skaffold Schema
use std::collections::HashMap;

/// Holds the fields parsed from Amphitheatre configuration file (amp.yaml).
#[derive(Default)]
pub struct Configuration<'r> {
    // The version of the configuration.
    pub api_version: String,

    // Kind is always `Config`. Defaults to `Config`.
    pub kind: &'r str,

    // Metadata holds additional information about the config.
    pub metadata: Metadata,

    // Partners describes a list of other required configs for the current
    // config.
    pub partners: Option<Vec<PartnerConfig>>,

    // Pipeline defines the Build/Test/Deploy phases.
    pub pipeline: Option<Pipeline>,

    // Profiles *beta* can override be used to `build`, `test` or `deploy`
    // configuration.
    pub profiles: Option<Vec<Profile>>,
}

/// Holds an optional name of the project
#[derive(Default)]
pub struct Metadata {
    // Name is an identifier for the project.
    pub name: String,

    // Labels is a map of labels identifying the project.
    pub labels: Option<HashMap<String, String>>,

    // Annotations is a map of annotations providing additional metadata about
    // the project.
    pub annotations: Option<HashMap<String, String>>,
}

/// PartnerConfig describes a dependency on another configuration.
#[derive(Default)]
pub struct PartnerConfig {
    // Names includes specific named configs within the file path. If empty,
    // then all configs in the file are included.
    pub names: Vec<String>,

    // Path describes the path to the file containing the required configs.
    pub path: String,

    // GitRepo describes a remote git repository containing the required
    // configs.
    pub git: Option<GitInfo>,

    // ActiveProfiles describes the list of profiles to activate when resolving
    // the partner configs. These profiles must exist in the imported config.
    pub active_profiles: Option<Vec<PartnerProfile>>,
}

/// GitInfo contains information on the origin of Amphitheatre configurations
/// cloned from a git repository.
pub struct GitInfo {
    // Repo is the git repository the package should be cloned from. e.g.
    //  `https://github.com/amphitheatre-app/amphitheatre.git`.
    pub repo: String,

    // Path is the relative path from the repo root to the Amphitheatre
    // configuration file. eg. `getting-started/amp.yaml`.
    pub path: Option<String>,

    // Ref is the git ref the package should be cloned from. eg. `master` or
    // `main`.
    pub reference: Option<String>,

    // Sync when set to `true` will reset the cached repository to the latest
    // commit from remote on every run. To use the cached repository with
    // uncommitted changes or unpushed commits, it needs to be set to `false`.
    pub sync: Option<bool>,
}

/// PartnerProfile describes a mapping from referenced config profiles to the
/// current config profiles. If the current config is activated with a profile in
/// this mapping then the partner configs are also activated with the
/// corresponding mapped profiles.
pub struct PartnerProfile {
    // Name describes name of the profile to activate in the partner config. It
    // should exist in the partner config.
    pub name: String,

    // ActivatedBy describes a list of profiles in the current config that when
    // activated will also activate the named profile in the partner config. If
    // empty then the named profile is always activated.
    pub activated_by: Option<Vec<String>>,
}

/// Pipeline describes a Amphitheatre pipeline.
#[derive(Default)]
pub struct Pipeline {
    // Build describes how images are built.
    pub build: Option<BuildConfig>,

    // Test describes how images are tested.
    pub test: Option<TestCase>,

    // Manifests describes how the original manifest are hydrated, validated and
    // transformed.
    pub manifests: Option<RenderConfig>,

    // Deploy describes how the manifest are deployed.
    pub deploy: Option<DeployConfig>,

    // Forward describes user defined resources to port-forward.
    pub forward: Option<PortForwardResource>,

    // Selector describes user defined filters describing how Amphitheatre
    // should treat objects/fields during rendering.
    pub selector: Option<ResourceSelectorConfig>,

    // Verify describes how images are verified (via verification tests).
    pub verify: Option<Vec<VerifyTestCase>>,
}

/// BuildConfig contains all the configuration for the build steps.
#[derive(Default)]
pub struct BuildConfig {
    // Artifacts lists the images you're going to be building.
    pub artifacts: Option<Vec<Artifact>>,

    // InsecureRegistries is a list of registries declared by the user to be
    // insecure. These registries will be connected to via HTTP instead of
    // HTTPS.
    pub insecure_registries: Option<Vec<String>>,

    // TagPolicy *beta* determines how images are tagged. A few strategies are
    // provided here, although you most likely won't need to care! If not
    // specified, it defaults to `gitCommit: {variant: Tags}`.
    pub tag_policy: Option<TagPolicy>,

    // Platforms is the list of platforms to build all artifact images for. It
    // can be overridden by the individual artifact's `platforms` property. If
    // the target builder cannot build for atleast one of the specified
    // platforms, then the build fails. Each platform is of the format
    // `os[/arch[/variant]]`, e.g., `linux/amd64`. Example: `["linux/amd64",
    // "linux/arm64"]`.
    pub platforms: Option<Vec<String>>,

    pub build_type: Option<BuildType>,
}

/// Artifact are the items that need to be built, along with the context in
/// which they should be built.
pub struct Artifact {
    // ImageName is the name of the image to be built. For example:
    // `gcr.io/amphitheatre/example`.
    pub image: String,

    // Workspace is the directory containing the artifact's sources. Defaults to
    // `.`.
    pub workspace: Option<String>,

    // Sync *beta* lists local files synced to pods instead of triggering an
    // image build when modified. If no files are listed, sync all the files and
    // infer the destination. Defaults to `infer: ["**/*"]`.
    pub sync: Option<Synchronization>,

    // ArtifactType describes how to build an artifact.
    pub artifact_type: ArtifactType,

    // Requires describes build artifacts that this artifact depends on.
    pub requires: Option<Vec<ArtifactDependency>>,

    // Hooks describes a set of lifecycle hooks that are executed before and
    // after each build of the target artifact.
    pub hooks: Option<BuildHooks>,

    // Platforms is the list of platforms to build this artifact image for. It
    // overrides the values inferred through heuristics or provided in the top
    // level `platforms` property or in the global config. If the target builder
    // cannot build for atleast one of the specified platforms, then the build
    // fails. Each platform is of the format `os[/arch[/variant]]`, e.g.,
    // `linux/amd64`. Example: `["linux/amd64", "linux/arm64"]`.
    pub platforms: Option<Vec<String>>,
}

/// Synchronization *beta* specifies what files to sync into the container. This
/// is a list of sync rules indicating the intent to sync for source files. If no
/// files are listed, sync all the files and infer the destination. Defaults to
/// `infer: ["**/*"]`.
pub struct Synchronization {
    // Manual lists manual sync rules indicating the source and destination.
    pub manual: Option<Vec<SyncRule>>,

    // Infer lists file patterns which may be synced into the container The
    // container destination is inferred by the builder based on the
    // instructions of a Dockerfile. Available for docker and kaniko artifacts
    // and custom artifacts that declare dependencies on a dockerfile.
    pub infer: Option<Vec<String>>,

    // Auto delegates discovery of sync rules to the build system. Only
    // available for jib and buildpacks.
    pub auto: Option<bool>,

    // LifecycleHooks describes a set of lifecycle hooks that are executed
    // before and after each file sync action on the target artifact's
    // containers.
    pub hooks: Option<SyncHooks>,
}

/// SyncRule specifies which local files to sync to remote folders.
pub struct SyncRule {
    // Src is a glob pattern to match local paths against. Directories should be
    // delimited by `/` on all platforms. For example: `"css/**/*.css"`.
    pub src: String,

    // Dest is the destination path in the container where the files should be
    // synced to. For example: `"app/"`
    pub dest: String,

    // Strip specifies the path prefix to remove from the source path when
    // transplanting the files into the destination folder. For example:
    // `"css/"`
    pub strip: String,
}

/// SyncHooks describes the list of lifecycle hooks to execute before and after
/// each artifact sync step.
pub struct SyncHooks {
    // PreHooks describes the list of lifecycle hooks to execute *before* each
    // artifact sync step.
    pub before: Option<Vec<SyncHookItem>>,

    // PostHooks describes the list of lifecycle hooks to execute *after* each
    // artifact sync step.
    pub after: Option<Vec<SyncHookItem>>,
}

/// SyncHookItem describes a single lifecycle hook to execute before or after
/// each artifact sync step.
pub struct SyncHookItem {
    // Host describes a single lifecycle hook to run on the host machine.
    pub host: Option<HostHook>,

    // Container describes a single lifecycle hook to run on a container.
    pub container: Option<ContainerHook>,
}

/// DeployHooks describes the list of lifecycle hooks to execute before and
/// after each deployer step.
pub struct DeployHooks {
    // PreHooks describes the list of lifecycle hooks to execute *before* each
    // deployer step. Container hooks will only run if the container exists from
    // a previous deployment step (for instance the successive iterations of a
    // dev-loop during `amphitheatre dev`).
    pub before: Option<Vec<DeployHookItem>>,

    // PostHooks describes the list of lifecycle hooks to execute *after* each
    // deployer step.
    pub after: Option<Vec<DeployHookItem>>,
}

/// DeployHookItem describes a single lifecycle hook to execute before or after
/// each deployer step.
pub struct DeployHookItem {
    // HostHook describes a single lifecycle hook to run on the host machine.
    pub host: Option<HostHook>,

    // ContainerHook describes a single lifecycle hook to run on a container.
    pub container: Option<NamedContainerHook>,
}

/// HostHook describes a lifecycle hook definition to execute on the host
/// machine.
pub struct HostHook {
    // Command is the command to execute.
    pub command: Vec<String>,

    // OS is an optional slice of operating system names. If the host machine OS
    // is different, then it skips execution.
    pub os: Option<Vec<String>>,

    // Dir specifies the working directory of the command. If empty, the command
    // runs in the calling process's current directory.
    pub dir: Option<String>,
}

/// ContainerHook describes a lifecycle hook definition to execute on a
///  container. The container name is inferred from the scope in which this hook
///  is defined.
pub struct ContainerHook {
    // Command is the command to execute.
    pub command: Vec<String>,
}

/// NamedContainerHook describes a lifecycle hook definition to execute on a
/// named container.
pub struct NamedContainerHook {
    // ContainerHook describes a lifecycle hook definition to execute on a
    // container.
    pub container_hook: Option<ContainerHook>,

    // PodName is the name of the pod to execute the command in.
    pub pod_name: String,

    // ContainerName is the name of the container to execute the command in.
    pub container_name: Option<String>,
}

/// ArtifactType describes how to build an artifact.
pub struct ArtifactType {
    // DockerArtifact *beta* describes an artifact built from a Dockerfile.
    pub docker: Option<DockerArtifact>,

    // BazelArtifact *beta* requires bazel CLI to be installed and the sources
    // to contain [Bazel](https://bazel.build/) configuration files.
    pub bazel: Option<BazelArtifact>,

    // KoArtifact builds images using [ko](https://github.com/google/ko).
    pub ko: Option<KoArtifact>,

    // JibArtifact builds images using the [Jib plugins for Maven or
    // Gradle](https://github.com/GoogleContainerTools/jib/).
    pub jib: Option<JibArtifact>,

    // KanikoArtifact builds images using
    // [kaniko](https://github.com/GoogleContainerTools/kaniko).
    pub kaniko: Option<KanikoArtifact>,

    // BuildpackArtifact builds images using [Cloud Native
    // Buildpacks](https://buildpacks.io/).
    pub buildpacks: Option<BuildpackArtifact>,

    // CustomArtifact *beta* builds images using a custom build script written
    // by the user.
    pub custom: Option<CustomArtifact>,
}

/// DockerArtifact describes an artifact built from a Dockerfile, usually using
/// `docker build`.
pub struct DockerArtifact {
    // DockerfilePath locates the Dockerfile relative to workspace. Defaults to
    // `Dockerfile`.
    pub dockerfile: Option<String>,

    // Target is the Dockerfile target name to build.
    pub target: Option<String>,

    // BuildArgs are arguments passed to the docker build. For example:
    // `{"key1": "value1", "key2": "{{ .ENV_VAR }}"}`.
    pub build_args: Option<HashMap<String, String>>,

    // NetworkMode is passed through to docker and overrides the network
    // configuration of docker builder. If unset, use whatever is configured in
    // the underlying docker daemon. Valid modes are `host`: use the host's
    // networking stack. `bridge`: use the bridged network configuration.
    // `container:<name|id>`: reuse another container's network stack. `none`:
    // no networking in the container.
    pub network: Option<String>,

    // AddHost lists add host. For example: `["host1:ip1", "host2:ip2"]`.
    pub add_host: Option<Vec<String>>,

    // CacheFrom lists the Docker images used as cache sources. For example:
    // `["golang:1.10.1-alpine3.7", "alpine:3.7"]`.
    pub cache_from: Option<Vec<String>>,

    // CliFlags are any additional flags to pass to the local daemon during a
    // build. These flags are only used during a build through the Docker CLI.
    pub cli_flags: Option<Vec<String>>,

    // PullParent is used to attempt pulling the parent image even if an older
    // image exists locally.
    pub pull_parent: Option<bool>,

    // NoCache set to true to pass in --no-cache to docker build, which will
    // prevent caching.
    pub no_cache: Option<bool>,

    // Squash is used to pass in --squash to docker build to squash docker image
    // layers into single layer.
    pub squash: Option<bool>,

    // Secrets is used to pass in --secret to docker build, `useBuildKit: true`
    // is required.
    pub secrets: Option<Vec<DockerSecret>>,

    // SSH is used to pass in --ssh to docker build to use SSH agent. Format is
    // "default|<id>[=<socket>|<key>[,<key>]]".
    pub ssh: Option<String>,
}

/// DockerSecret is used to pass in --secret to docker build, `useBuildKit:
/// true` is required.
pub struct DockerSecret {
    // ID is the id of the secret.
    pub id: String,

    // Source is the path to the secret on the host machine.
    pub src: Option<String>,

    // Env is the environment variable name containing the secret value.
    pub env: Option<String>,
}

/// BazelArtifact describes an artifact built with
/// [Bazel](https://bazel.build/).
pub struct BazelArtifact {
    // BuildTarget is the `bazel build` target to run. For example:
    // `//:amphitheatre_example.tar`.
    pub target: String,

    // BuildArgs are additional args to pass to `bazel build`. For example:
    // `["-flag", "--otherflag"]`.
    pub args: Option<Vec<String>>,
}

/// KoArtifact builds images using [ko](https://github.com/google/ko).
pub struct KoArtifact {
    // BaseImage overrides the default ko base image
    // (`gcr.io/distroless/static:nonroot`). Corresponds to, and overrides, the
    // `defaultBaseImage` in `.ko.yaml`.
    pub from_image: Option<String>,

    // Dependencies are the file dependencies that Amphitheatre should watch for
    // both rebuilding and file syncing for this artifact.
    pub dependencies: Option<KoDependencies>,

    // Dir is the directory where the `go` tool will be run. The value is a
    // directory path relative to the `context` directory. If empty, the `go`
    // tool will run in the `context` directory. Example: `./my-app-sources`.
    pub dir: Option<String>,

    // Env are environment variables, in the `key=value` form, passed to the
    // build. These environment variables are only used at build time. They are
    // _not_ set in the resulting container image. For example:
    // `["GOPRIVATE=git.example.com", "GOCACHE=/workspace/.gocache"]`.
    pub env: Option<Vec<String>>,

    // Flags are additional build flags passed to `go build`. For example:
    // `["-trimpath", "-v"]`.
    pub flags: Option<Vec<String>>,

    // Labels are key-value string pairs to add to the image config. For
    // example: `{"foo":"bar"}`.
    pub labels: Option<HashMap<String, String>>,

    // Ldflags are linker flags passed to the builder. For example:
    // `["-buildid=", "-s", "-w"]`.
    pub ldflags: Option<Vec<String>>,

    // Main is the location of the main package. It is the pattern passed to `go
    // build`. If main is specified as a relative path, it is relative to the
    // `context` directory. If main is empty, the ko builder uses a default
    // value of `.`. If main is a pattern with wildcards, such as `./...`, the
    // expansion must contain only one main package, otherwise ko fails. Main is
    // ignored if the `ImageName` starts with `ko://`. Example: `./cmd/foo`.
    pub main: Option<String>,
}

/// KoDependencies is used to specify dependencies for an artifact built by ko.
pub struct KoDependencies {
    // Paths should be set to the file dependencies for this artifact, so that
    // the Amphitheatre file watcher knows when to rebuild and perform file
    // synchronization. Defaults to `["**/*.go"]`.
    pub paths: Option<Vec<String>>,

    // Ignore specifies the paths that should be ignored by Amphitheatre's file
    // watcher. If a file exists in both `paths` and in `ignore`, it will be
    // ignored, and will be excluded from both rebuilds and file
    // synchronization.
    pub ignore: Option<Vec<String>>,
}

/// JibArtifact builds images using the [Jib plugins for Maven and
/// Gradle](https://github.com/GoogleContainerTools/jib/).
pub struct JibArtifact {
    // Project selects which sub-project to build for multi-module builds.
    pub project: Option<String>,

    // Flags are additional build flags passed to the builder. For example:
    // `["--no-build-cache"]`.
    pub args: Option<Vec<String>>,

    // Type the Jib builder type; normally determined automatically. Valid types
    // are `maven`: for Maven. `gradle`: for Gradle.
    pub builder_type: Option<String>,

    // BaseImage overrides the configured jib base image.
    pub from_image: Option<String>,
}

/// KanikoArtifact describes an artifact built from a Dockerfile, with kaniko.
pub struct KanikoArtifact {
    // Cleanup to clean the filesystem at the end of the build.
    pub cleanup: Option<bool>,

    // Insecure if you want to push images to a plain HTTP registry.
    pub insecure: Option<bool>,

    // InsecurePull if you want to pull images from a plain HTTP registry.
    pub insecure_pull: Option<bool>,

    // NoPush if you only want to build the image, without pushing to a
    // registry.
    pub no_push: Option<bool>,

    // Force building outside of a container.
    pub force: Option<bool>,

    // LogTimestamp to add timestamps to log format.
    pub log_timestamp: Option<bool>,

    // Reproducible is used to strip timestamps out of the built image.
    pub reproducible: Option<bool>,

    // SingleSnapshot is takes a single snapshot of the filesystem at the end of
    // the build. So only one layer will be appended to the base image.
    pub single_snapshot: Option<bool>,

    // SkipTLS skips TLS certificate validation when pushing to a registry.
    pub skip_tls: Option<bool>,

    // SkipTLSVerifyPull skips TLS certificate validation when pulling from a
    // registry.
    pub skip_tlsverify_pull: Option<bool>,

    // SkipUnusedStages builds only used stages if defined to true. Otherwise it
    // builds by default all stages, even the unnecessaries ones until it
    // reaches the target stage / end of Dockerfile.
    pub skip_unused_stages: Option<bool>,

    // UseNewRun to Use the experimental run implementation for detecting
    // changes without requiring file system snapshots. In some cases, this may
    // improve build performance by 75%.
    pub use_new_run: Option<bool>,

    // WhitelistVarRun is used to ignore `/var/run` when taking image snapshot.
    // Set it to false to preserve /var/run/* in destination image.
    pub whitelist_var_run: Option<bool>,

    // DockerfilePath locates the Dockerfile relative to workspace. Defaults to
    // `Dockerfile`.
    pub dockerfile: Option<String>,

    // Target is to indicate which build stage is the target build stage.
    pub target: Option<String>,

    // InitImage is the image used to run init container which mounts kaniko
    // context.
    pub init_image: Option<String>,

    // Image is the Docker image used by the Kaniko pod. Defaults to the latest
    // released version of `gcr.io/kaniko-project/executor`.
    pub image: Option<String>,

    // DigestFile to specify a file in the container. This file will receive the
    // digest of a built image. This can be used to automatically track the
    // exact image built by kaniko.
    pub digest_file: Option<String>,

    // ImageFSExtractRetry is the number of retries that should happen for
    // extracting an image filesystem.
    pub image_fs_extract_retry: Option<String>,

    // ImageNameWithDigestFile specify a file to save the image name with digest
    // of the built image to.
    pub image_name_with_digest_file: Option<String>,

    // LogFormat <text|color|json> to set the log format.
    pub log_format: Option<String>,

    // OCILayoutPath is to specify a directory in the container where the OCI
    // image layout of a built image will be placed. This can be used to
    // automatically track the exact image built by kaniko.
    pub oci_layout_path: Option<String>,

    // RegistryMirror if you want to use a registry mirror instead of default
    // `index.docker.io`.
    pub registry_mirror: Option<String>,

    // SnapshotMode is how Kaniko will snapshot the filesystem.
    pub snapshot_mode: Option<String>,

    // PushRetry Set this flag to the number of retries that should happen for
    // the push of an image to a remote destination.
    pub push_retry: Option<String>,

    // TarPath is path to save the image as a tarball at path instead of pushing
    // the image.
    pub tar_path: Option<String>,

    // Verbosity <panic|fatal|error|warn|info|debug|trace> to set the logging
    // level.
    pub verbosity: Option<String>,

    // InsecureRegistry is to use plain HTTP requests when accessing a registry.
    pub insecure_registry: Option<Vec<String>>,

    // SkipTLSVerifyRegistry skips TLS certificate validation when accessing a
    // registry.
    pub skip_tlsverify_registry: Option<Vec<String>>,

    // Env are environment variables passed to the kaniko pod. It also accepts
    // environment variables via the go template syntax. For example: `[{"name":
    // "key1", "value": "value1"}, {"name": "key2", "value": "value2"}, {"name":
    // "key3", "value": "'{{.ENV_VARIABLE}}'"}]`.
    pub env: Option<Vec<EnvVar>>,

    // Cache configures Kaniko caching. If a cache is specified, Kaniko will use
    // a remote cache which will speed up builds.
    pub cache: Option<KanikoCache>,

    // RegistryCertificate is to provide a certificate for TLS communication
    // with a given registry. my.registry.url: /path/to/the/certificate.cert is
    // the expected format.
    pub registry_certificate: Option<HashMap<String, String>>,

    // Label key: value to set some metadata to the final image. This is
    // equivalent as using the LABEL within the Dockerfile.
    pub label: Option<HashMap<String, String>>,

    // BuildArgs are arguments passed to the docker build. It also accepts
    // environment variables and generated values via the go template syntax.
    // Exposed generated values: IMAGE_REPO, IMAGE_NAME, IMAGE_TAG. For example:
    // `{"key1": "value1", "key2": "value2", "key3": "'{{.ENV_VARIABLE}}'"}`.
    pub build_args: Option<HashMap<String, String>>,

    // VolumeMounts are volume mounts passed to kaniko pod.
    pub volume_mounts: Option<Vec<VolumeMount>>,

    // ContextSubPath is to specify a sub path within the context.
    pub context_sub_path: Option<String>,
}

/// @TODO: v1.EnvVar
pub struct EnvVar {}

/// KanikoCache configures Kaniko caching. If a cache is specified, Kaniko will
/// use a remote cache which will speed up builds.
pub struct KanikoCache {
    // Repo is a remote repository to store cached layers. If none is specified,
    // one will be inferred from the image name. See [Kaniko
    // Caching](https://github.com/GoogleContainerTools/kaniko#caching).
    pub repo: Option<String>,

    // HostPath specifies a path on the host that is mounted to each pod as read
    // only cache volume containing base images. If set, must exist on each node
    // and prepopulated with kaniko-warmer.
    pub host_path: Option<String>,

    // TTL Cache timeout in hours.
    pub ttl: Option<String>,

    // CacheCopyLayers enables caching of copy layers.
    pub cache_copy_layers: Option<bool>,
}

/// BuildpackArtifact *alpha* describes an artifact built using [Cloud Native
/// Buildpacks](https://buildpacks.io/). It can be used to build images out of
/// project's sources without any additional configuration.
pub struct BuildpackArtifact {
    // Builder is the builder image used.
    pub builder: String,

    // RunImage overrides the stack's default run image.
    pub run_image: Option<String>,

    // Env are environment variables, in the `key=value` form,  passed to the
    // build. Values can use the go template syntax. For example:
    // `["key1=value1", "key2=value2", "key3={{.ENV_VARIABLE}}"]`.
    pub env: Option<Vec<String>>,

    // Buildpacks is a list of strings, where each string is a specific
    // buildpack to use with the builder. If you specify buildpacks the builder
    // image automatic detection will be ignored. These buildpacks will be used
    // to build the Image from your source code. Order matters.
    pub buildpacks: Option<Vec<String>>,

    // TrustBuilder indicates that the builder should be trusted.
    pub trust_builder: Option<bool>,

    // ProjectDescriptor is the path to the project descriptor file. Defaults to
    // `project.toml` if it exists.
    pub project_descriptor: Option<String>,

    // Dependencies are the file dependencies that amphitheatre should watch for
    // both rebuilding and file syncing for this artifact.
    pub dependencies: Option<BuildpackDependencies>,

    // Volumes support mounting host volumes into the container.
    pub volumes: Option<Vec<BuildpackVolume>>,
}

/// BuildpackDependencies *alpha* is used to specify dependencies for an
/// artifact built by buildpacks.
pub struct BuildpackDependencies {
    // Paths should be set to the file dependencies for this artifact, so that
    // the amphitheatre file watcher knows when to rebuild and perform file
    // synchronization.
    pub paths: Option<Vec<String>>,

    // Ignore specifies the paths that should be ignored by amphitheatre's file
    // watcher. If a file exists in both `paths` and in `ignore`, it will be
    // ignored, and will be excluded from both rebuilds and file
    // synchronization. Will only work in conjunction with `paths`.
    pub ignore: Option<Vec<String>>,
}

/// BuildpackVolume *alpha* is used to mount host volumes or directories in the
/// build container.
pub struct BuildpackVolume {
    // Host is the local volume or absolute directory of the path to mount.
    pub host: String,

    // Target is the path where the file or directory is available in the
    // container. It is strongly recommended to not specify locations under
    // `/cnb` or `/layers`.
    pub target: String,

    // Options specify a list of comma-separated mount options. Valid options
    // are: `ro` (default): volume contents are read-only. `rw`: volume contents
    // are readable and writable. `volume-opt=<key>=<value>`: can be specified
    // more than once, takes a key-value pair.
    pub options: Option<String>,
}

/// CustomArtifact *beta* describes an artifact built from a custom build script
/// written by the user. It can be used to build images with builders that
/// aren't directly integrated with amphitheatre.
pub struct CustomArtifact {
    // BuildCommand is the command executed to build the image.
    pub build_command: Option<String>,

    // Dependencies are the file dependencies that amphitheatre should watch for
    // both rebuilding and file syncing for this artifact.
    pub dependencies: Option<CustomDependencies>,
}

/// CustomDependencies *beta* is used to specify dependencies for an artifact
/// built by a custom build script. Either `dockerfile` or `paths` should be
/// specified for file watching to work as expected.
pub struct CustomDependencies {
    // Dockerfile should be set if the artifact is built from a Dockerfile, from
    // which amphitheatre can determine dependencies.
    pub dockerfile: Option<DockerfileDependency>,

    // Command represents a custom command that amphitheatre executes to obtain
    // dependencies. The output of this command *must* be a valid JSON array.
    pub command: Option<String>,

    // Paths should be set to the file dependencies for this artifact, so that
    // the amphitheatre file watcher knows when to rebuild and perform file
    // synchronization.
    pub paths: Option<Vec<String>>,

    // Ignore specifies the paths that should be ignored by amphitheatre's file
    // watcher. If a file exists in both `paths` and in `ignore`, it will be
    // ignored, and will be excluded from both rebuilds and file
    // synchronization. Will only work in conjunction with `paths`.
    pub ignore: Option<Vec<String>>,
}

/// DockerfileDependency *beta* is used to specify a custom build artifact that
/// is built from a Dockerfile. This allows amphitheatre to determine
/// dependencies from the Dockerfile.
pub struct DockerfileDependency {
    // Path locates the Dockerfile relative to workspace.
    pub path: Option<String>,

    // BuildArgs are key/value pairs used to resolve values of `ARG`
    // instructions in a Dockerfile. Values can be constants or environment
    // variables via the go template syntax. For example: `{"key1": "value1",
    // "key2": "value2", "key3": "'{{.ENV_VARIABLE}}'"}`.
    pub build_args: Option<HashMap<String, String>>,
}

/// ArtifactDependency describes a specific build dependency for an artifact.
pub struct ArtifactDependency {
    // ImageName is a reference to an artifact's image name.
    pub image: String,

    // Alias is a token that is replaced with the image reference in the builder
    // definition files. For example, the `docker` builder will use the alias as
    // a build-arg key. Defaults to the value of `image`.
    pub alias: Option<String>,
}

/// BuildHooks describes the list of lifecycle hooks to execute before and after
/// each artifact build step.
pub struct BuildHooks {
    // PreHooks describes the list of lifecycle hooks to execute *before* each
    // artifact build step.
    pub before: Option<Vec<HostHook>>,

    // PostHooks describes the list of lifecycle hooks to execute *after* each
    // artifact build step.
    pub after: Option<Vec<HostHook>>,
}

/// TagPolicy contains all the configuration for the tagging step.
pub struct TagPolicy {
    // GitTagger *beta* tags images with the git tag or commit of the artifact's
    // workspace.
    pub git_commit: Option<GitTagger>,

    // ShaTagger *beta* tags images with their sha256 digest.
    pub sha256: Option<ShaTagger>,

    // EnvTemplateTagger *beta* tags images with a configurable template string.
    pub env_template: Option<EnvTemplateTagger>,

    // DateTimeTagger *beta* tags images with the build timestamp.
    pub datetime: Option<DateTimeTagger>,

    // CustomTemplateTagger *beta* tags images with a configurable template
    // string *composed of other taggers*.
    pub custom_template: Option<CustomTemplateTagger>,

    // InputDigest *beta* tags images with their sha256 digest of their content.
    pub input_digest: Option<InputDigest>,
}

/// GitTagger *beta* tags images with the git tag or commit of the artifact's
/// workspace.
pub struct GitTagger {
    // Variant determines the behavior of the git tagger. Valid variants are:
    // `Tags` (default): use git tags or fall back to abbreviated commit hash.
    // `CommitSha`: use the full git commit sha. `AbbrevCommitSha`: use the
    // abbreviated git commit sha. `TreeSha`: use the full tree hash of the
    // artifact workingdir. `AbbrevTreeSha`: use the abbreviated tree hash of
    // the artifact workingdir.
    pub variant: Option<String>,

    // Prefix adds a fixed prefix to the tag.
    pub prefix: Option<String>,

    // IgnoreChanges specifies whether to omit the `-dirty` postfix if there are
    // uncommitted changes.
    pub ignore_changes: Option<bool>,
}

/// ShaTagger *beta* tags images with their sha256 digest.
pub struct ShaTagger {}

/// EnvTemplateTagger *beta* tags images with a configurable template string.
pub struct EnvTemplateTagger {
    // Template used to produce the image name and tag. See golang
    // [text/template](https://golang.org/pkg/text/template/). The template is
    // executed against the current environment, with those variables injected.
    // For example: `{{.RELEASE}}`.
    pub template: String,
}

/// DateTimeTagger *beta* tags images with the build timestamp.
pub struct DateTimeTagger {
    // Format formats the date and time. See
    // [#Time.Format](https://golang.org/pkg/time/#Time.Format). Defaults to
    // `2006-01-02_15-04-05.999_MST`.
    pub format: Option<String>,

    // TimeZone sets the timezone for the date and time. See
    // [Time.LoadLocation](https://golang.org/pkg/time/#Time.LoadLocation).
    // Defaults to the local timezone.
    pub timezone: Option<String>,
}

/// CustomTemplateTagger *beta* tags images with a configurable template string.
pub struct CustomTemplateTagger {
    // Template used to produce the image name and tag. See golang
    // [text/template](https://golang.org/pkg/text/template/). The template is
    // executed against the provided components with those variables injected.
    // For example: `{{.DATE}}` where DATE references a TaggerComponent.
    pub template: String,

    // Components lists TaggerComponents that the template (see field above) can
    // be executed against.
    pub components: Option<Vec<TaggerComponent>>,
}

/// TaggerComponent *beta* is a component of CustomTemplateTagger.
pub struct TaggerComponent {
    // Name is an identifier for the component.
    pub name: Option<String>,

    // Component is a tagging strategy to be used in CustomTemplateTagger.
    pub component: Option<TagPolicy>,
}

/// InputDigest *beta* tags hashes the image content.
pub struct InputDigest {}

/// BuildType contains the specific implementation and parameters needed for the
/// build step. Only one field should be populated.
pub struct BuildType {
    // LocalBuild *beta* describes how to do a build on the local docker daemon
    // and optionally push to a repository.
    pub local: Option<LocalBuild>,

    // GoogleCloudBuild *beta* describes how to do a remote build on [Google
    // Cloud Build](https://cloud.google.com/cloud-build/).
    pub google_cloud_build: Option<GoogleCloudBuild>,

    // Cluster *beta* describes how to do an on-cluster build.
    pub cluster: Option<ClusterDetails>,
}

/// LocalBuild *beta* describes how to do a build on the local docker daemon and
/// optionally push to a repository.
pub struct LocalBuild {
    // Push should images be pushed to a registry. If not specified, images are
    // pushed only if the current Kubernetes context connects to a remote
    // cluster.
    pub push: Option<bool>,

    // TryImportMissing whether to attempt to import artifacts from Docker
    // (either a local or remote registry) if not in the cache.
    pub try_import_missing: Option<bool>,

    // UseDockerCLI use `docker` command-line interface instead of Docker Engine
    // APIs.
    pub use_docker_cli: Option<bool>,

    // UseBuildkit use BuildKit to build Docker images. If unspecified, uses the
    // Docker default.
    pub use_buildkit: Option<bool>,

    // Concurrency is how many artifacts can be built concurrently. 0 means
    // "no-limit". Defaults to `1`.
    pub concurrency: Option<i32>,
}

/// GoogleCloudBuild *beta* describes how to do a remote build on [Google Cloud
/// Build](https://cloud.google.com/cloud-build/docs/). Docker and Jib artifacts
/// can be built on Cloud Build. The `projectId` needs to be provided and the
/// currently logged in user should be given permissions to trigger new builds.
pub struct GoogleCloudBuild {
    // ProjectID is the ID of your Cloud Platform Project. If it is not
    // provided, Amphitheatre will guess it from the image name. For example,
    // given the artifact image name `gcr.io/myproject/image`, Amphitheatre will
    // use the `myproject` GCP project.
    pub project_id: Option<String>,

    // DiskSizeGb is the disk size of the VM that runs the build. See [Cloud
    // Build
    // Reference](https://cloud.google.com/cloud-build/docs/api/reference/rest/v1/projects.builds#buildoptions).
    pub disk_size_gb: Option<i64>,

    // MachineType is the type of the VM that runs the build. See [Cloud Build
    // Reference](https://cloud.google.com/cloud-build/docs/api/reference/rest/v1/projects.builds#buildoptions).
    pub machine_type: Option<String>,

    // Timeout is the amount of time (in seconds) that this build should be
    // allowed to run. See [Cloud Build
    // Reference](https://cloud.google.com/cloud-build/docs/api/reference/rest/v1/projects.builds#resource-build).
    pub timeout: Option<String>,

    // Logging specifies the logging mode. Valid modes are:
    // `LOGGING_UNSPECIFIED`: The service determines the logging mode. `LEGACY`:
    // Stackdriver logging and Cloud Storage logging are enabled (default).
    // `GCS_ONLY`: Only Cloud Storage logging is enabled. See [Cloud Build
    // Reference](https://cloud.google.com/cloud-build/docs/api/reference/rest/v1/projects.builds#loggingmode).
    pub logging: Option<String>,

    // LogStreamingOption specifies the behavior when writing build logs to
    // Google Cloud Storage. Valid options are: `STREAM_DEFAULT`: Service may
    // automatically determine build log streaming behavior. `STREAM_ON`:  Build
    // logs should be streamed to Google Cloud Storage. `STREAM_OFF`: Build logs
    // should not be streamed to Google Cloud Storage; they will be written when
    // the build is completed. See [Cloud Build
    // Reference](https://cloud.google.com/cloud-build/docs/api/reference/rest/v1/projects.builds#logstreamingoption).
    pub log_streaming_option: Option<String>,

    // DockerImage is the image that runs a Docker build. See [Cloud
    // Builders](https://cloud.google.com/cloud-build/docs/cloud-builders).
    // Defaults to `gcr.io/cloud-builders/docker`.
    pub docker_image: Option<String>,

    // KanikoImage is the image that runs a Kaniko build. See [Cloud
    // Builders](https://cloud.google.com/cloud-build/docs/cloud-builders).
    // Defaults to `gcr.io/kaniko-project/executor`.
    pub kaniko_image: Option<String>,

    // MavenImage is the image that runs a Maven build. See [Cloud
    // Builders](https://cloud.google.com/cloud-build/docs/cloud-builders).
    // Defaults to `gcr.io/cloud-builders/mvn`.
    pub maven_image: Option<String>,

    // GradleImage is the image that runs a Gradle build. See [Cloud
    // Builders](https://cloud.google.com/cloud-build/docs/cloud-builders).
    // Defaults to `gcr.io/cloud-builders/gradle`.
    pub gradle_image: Option<String>,

    // PackImage is the image that runs a Cloud Native Buildpacks build. See
    // [Cloud
    // Builders](https://cloud.google.com/cloud-build/docs/cloud-builders).
    // Defaults to `gcr.io/amphitheatre/pack`.
    pub pack_image: Option<String>,

    // Concurrency is how many artifacts can be built concurrently. 0 means
    // "no-limit". Defaults to `0`.
    pub concurrency: Option<i32>,

    // WorkerPool configures a pool of workers to run the build.
    pub worker_pool: Option<String>,

    // Region configures the region to run the build. If WorkerPool is
    // configured, the region will be deduced from the WorkerPool configuration.
    // If neither WorkerPool nor Region is configured, the build will be run in
    // global(non-regional). See [Cloud Build
    // locations](https://cloud.google.com/build/docs/locations)
    pub region: Option<String>,
}

/// ClusterDetails *beta* describes how to do an on-cluster build.
pub struct ClusterDetails {
    // HTTPProxy for kaniko pod.
    pub http_proxy: Option<String>,

    // HTTPSProxy for kaniko pod.
    pub https_proxy: Option<String>,

    // PullSecretPath is the path to the Google Cloud service account secret key
    // file.
    pub pull_secret_path: Option<String>,

    // PullSecretName is the name of the Kubernetes secret for pulling base
    // images and pushing the final image. If given, the secret needs to contain
    // the Google Cloud service account secret key under the key
    // `kaniko-secret`. Defaults to `kaniko-secret`.
    pub pull_secret_name: Option<String>,

    // PullSecretMountPath is the path the pull secret will be mounted at within
    // the running container.
    pub pull_secret_mount_path: Option<String>,

    // Namespace is the Kubernetes namespace. Defaults to current namespace in
    // Kubernetes configuration.
    pub namespace: Option<String>,

    // Timeout is the amount of time (in seconds) that this build is allowed to
    // run. Defaults to 20 minutes (`20m`).
    pub timeout: Option<String>,

    // DockerConfig describes how to mount the local Docker configuration into a
    // pod.
    pub docker_config: Option<DockerConfig>,

    // ServiceAccountName describes the Kubernetes service account to use for
    // the pod. Defaults to 'default'.
    pub service_account: Option<String>,

    // Tolerations describes the Kubernetes tolerations for the pod.
    pub tolerations: Option<Vec<Toleration>>,

    // NodeSelector describes the Kubernetes node selector for the pod.
    pub node_selector: Option<HashMap<String, String>>,

    // Annotations describes the Kubernetes annotations for the pod.
    pub annotations: Option<HashMap<String, String>>,

    // RunAsUser defines the UID to request for running the container. If
    // omitted, no SecurityContext will be specified for the pod and will
    // therefore be inherited from the service account.
    pub run_as_user: Option<i64>,

    // Resources define the resource requirements for the kaniko pod.
    pub resources: Option<ResourceRequirements>,

    // Concurrency is how many artifacts can be built concurrently. 0 means
    // "no-limit". Defaults to `0`.
    pub concurrency: Option<i32>,

    // Volumes defines container mounts for ConfigMap and Secret resources.
    pub volumes: Option<Vec<Volume>>,

    // RandomPullSecret adds a random UUID postfix to the default name of the
    // pull secret to facilitate parallel builds, e.g.
    // kaniko-secretdocker-cfgfd154022-c761-416f-8eb3-cf8258450b85.
    pub random_pull_secret: Option<bool>,

    // RandomDockerConfigSecret adds a random UUID postfix to the default name
    // of the docker secret to facilitate parallel builds, e.g.
    // docker-cfgfd154022-c761-416f-8eb3-cf8258450b85.
    pub random_docker_config_secret: Option<bool>,
}

/// @TODO: v1.Toleration
pub struct Toleration {}

/// @TODO: v1.Volume
pub struct Volume {}

/// @TODO: v1.VolumeMount
pub struct VolumeMount {}

/// DockerConfig contains information about the docker `config.json` to mount.
pub struct DockerConfig {
    // Path is the path to the docker `config.json`.
    pub path: Option<String>,

    // SecretName is the Kubernetes secret that contains the `config.json`
    // Docker configuration. Note that the expected secret type is not
    // 'kubernetes.io/dockerconfigjson' but 'Opaque'.
    pub secret_name: Option<String>,
}

/// ResourceRequirements describes the resource requirements for the kaniko pod.
pub struct ResourceRequirements {
    // Requests [resource
    // requests](https://kubernetes.io/docs/concepts/configuration/manage-compute-resources-container/#resource-requests-and-limits-of-pod-and-container)
    // for the Kaniko pod.
    pub requests: Option<ResourceRequirement>,

    // Limits [resource
    // limits](https://kubernetes.io/docs/concepts/configuration/manage-compute-resources-container/#resource-requests-and-limits-of-pod-and-container)
    // for the Kaniko pod.
    pub limits: Option<ResourceRequirement>,
}

/// ResourceRequirement stores the CPU/Memory requirements for the pod.
pub struct ResourceRequirement {
    // CPU the number cores to be used. For example: `2`, `2.0` or `200m`.
    pub cpu: Option<String>,

    // Memory the amount of memory to allocate to the pod. For example: `1Gi` or
    // `1000Mi`.
    pub memory: Option<String>,

    // EphemeralStorage the amount of Ephemeral storage to allocate to the pod.
    // For example: `1Gi` or `1000Mi`.
    pub ephemeral_storage: Option<String>,

    // ResourceStorage the amount of resource storage to allocate to the pod.
    // For example: `1Gi` or `1000Mi`.
    pub resource_storage: Option<String>,
}

/// TestCase is a list of tests to run on images that Amphitheatre builds.
#[derive(Default)]
pub struct TestCase {
    // ImageName is the artifact on which to run those tests. For example:
    // `gcr.io/amphitheatre/example`.
    pub image: String,

    // Workspace is the directory containing the test sources. Defaults to `.`.
    pub context: Option<String>,

    // CustomTests lists the set of custom tests to run after an artifact is
    // built.
    pub custom: Option<Vec<CustomTest>>,

    // StructureTests lists the [Container Structure
    // Tests](https://github.com/GoogleContainerTools/container-structure-test)
    // to run on that artifact. For example: `["./test/*"]`.
    pub structure_tests: Option<Vec<String>>,

    // StructureTestArgs lists additional configuration arguments passed to
    // `container-structure-test` binary. For example: `["--driver=tar",
    // "--no-color", "-q"]`.
    pub structure_tests_args: Option<Vec<String>>,
}

/// CustomTest describes the custom test command provided by the user. Custom
/// tests are run after an image build whenever build or test dependencies are
/// changed.
pub struct CustomTest {
    // Command is the custom command to be executed.  If the command exits with
    // a non-zero return code, the test will be considered to have failed.
    pub command: String,

    // TimeoutSeconds sets the wait time for amphitheatre for the command to
    // complete. If unset or 0, Amphitheatre will wait until the command
    // completes.
    pub timeout_seconds: Option<i32>,

    // Dependencies are additional test-specific file dependencies; changes to
    // these files will re-run this test.
    pub dependencies: Option<CustomTestDependencies>,
}

/// CustomTestDependencies is used to specify dependencies for custom test
/// command. `paths` should be specified for file watching to work as expected.
pub struct CustomTestDependencies {
    // Command represents a command that amphitheatre executes to obtain
    // dependencies. The output of this command *must* be a valid JSON array.
    pub command: Option<String>,

    // Paths locates the file dependencies for the command relative to
    // workspace. Paths should be set to the file dependencies for this command,
    // so that the amphitheatre file watcher knows when to retest and perform
    // file synchronization. For example: `["src/test/**"]`
    pub paths: Option<Vec<String>>,

    // Ignore specifies the paths that should be ignored by amphitheatre's file
    // watcher. If a file exists in both `paths` and in `ignore`, it will be
    // ignored, and will be excluded from both retest and file synchronization.
    // Will only work in conjunction with `paths`.
    pub ignore: Option<Vec<String>>,
}

/// RenderConfig contains all the configuration needed by the render steps.
#[derive(Default)]
pub struct RenderConfig {
    // Generate defines the dry manifests from a variety of sources.
    pub generate: Option<Generate>,

    // Transform defines a set of transformation operations to run in series.
    pub transform: Option<Vec<Transformer>>,

    // Validate defines a set of validator operations to run in series.
    pub validate: Option<Vec<Validator>>,

    // Output is the path to the hydrated directory.
    pub output: Option<String>,
}

/// Generate defines the dry manifests from a variety of sources.
pub struct Generate {
    pub raw_yaml: Option<Vec<String>>,

    // Kustomize defines the paths to be modified with kustomize, along with
    // extra flags to be passed to kustomize
    pub kustomize: Option<Kustomize>,

    pub helm: Option<Helm>,

    pub kpt: Option<Vec<String>>,
}

/// Kustomize defines the paths to be modified with kustomize, along with extra
/// flags to be passed to kustomize
pub struct Kustomize {
    // Paths is the path to Kustomization files. Defaults to `["."]`.
    pub paths: Option<Vec<String>>,

    // BuildArgs are additional args passed to `kustomize build`.
    pub build_args: Option<Vec<String>>,
}

/// Helm defines the manifests from helm releases.
pub struct Helm {
    // Flags are additional option flags that are passed on the command line to
    // `helm`.
    pub flags: Option<HelmDeployFlags>,

    // Releases is a list of Helm releases.
    pub releases: Vec<HelmRelease>,
}

/// Transformer describes the supported kpt transformers.
pub struct Transformer {
    // Name is the transformer name. Can only accept amphitheatre whitelisted
    // tools.
    pub name: String,

    // ConfigMap allows users to provide additional config data to the kpt
    // function.
    pub config_map: Option<Vec<String>>,
}

/// Validator describes the supported kpt validators.
pub struct Validator {
    // Name is the Validator name. Can only accept amphitheatre whitelisted
    // tools.
    pub name: String,

    // ConfigMap allows users to provide additional config data to the kpt
    // function.
    pub config_map: Option<Vec<String>>,
}

/// DeployConfig contains all the configuration needed by the deploy steps.
#[derive(Default)]
pub struct DeployConfig {
    pub deploy_type: Option<DeployType>,

    // StatusCheck *beta* enables waiting for deployments to stabilize.
    pub status_check: Option<bool>,

    // StatusCheckDeadlineSeconds *beta* is the deadline for deployments to
    // stabilize in seconds.
    pub status_check_deadline_seconds: Option<i32>,

    // KubeContext is the Kubernetes context that Amphitheatre should deploy to.
    // For example: `minikube`.
    pub kube_context: Option<String>,

    // Logs configures how container logs are printed as a result of a
    // deployment.
    pub logs: Option<LogsConfig>,

    // TransformableAllowList configures an allowlist for transforming
    // manifests.
    pub transformable_allow_list: Option<Vec<ResourceFilter>>,
}

/// DeployType contains the specific implementation and parameters needed for
/// the deploy step. All three deployer types can be used at the same time for
/// hybrid workflows.
pub struct DeployType {
    // DockerDeploy *alpha* uses the `docker` CLI to create application
    // containers in Docker.
    pub docker: Option<DockerDeploy>,

    // LegacyHelmDeploy *beta* uses the `helm` CLI to apply the charts to the
    // cluster.
    pub helm: Option<LegacyHelmDeploy>,

    // KptDeploy *alpha* uses the `kpt` CLI to manage and deploy manifests.
    pub kpt: Option<KptDeploy>,

    // KubectlDeploy *beta* uses a client side `kubectl apply` to deploy
    // manifests. You'll need a `kubectl` CLI version installed that's
    // compatible with your cluster.
    pub kubectl: Option<KubectlDeploy>,

    // KustomizeDeploy *beta* uses the `kustomize` CLI to "patch" a deployment
    // for a target environment.
    pub kustomize: Option<KustomizeDeploy>,

    // CloudRunDeploy *alpha* deploys to Google Cloud Run using the Cloud Run v1
    // API
    pub cloudrun: Option<CloudRunDeploy>,
}

/// DockerDeploy uses the `docker` CLI to create application containers in
/// Docker.
pub struct DockerDeploy {
    // UseCompose tells amphitheatre whether or not to deploy using
    // `docker-compose`.
    pub use_compose: Option<bool>,

    // Images are the container images to run in Docker.
    pub images: Vec<String>,
}

/// LegacyHelmDeploy *beta* uses the `helm` CLI to apply the charts to the
/// cluster.
pub struct LegacyHelmDeploy {
    // Releases is a list of Helm releases.
    pub releases: Option<Vec<HelmRelease>>,

    // Flags are additional option flags that are passed on the command line to
    // `helm`.
    pub flags: Option<HelmDeployFlags>,

    // LifecycleHooks describes a set of lifecycle hooks that are executed
    // before and after every deploy.
    pub hooks: Option<DeployHooks>,
}

/// HelmRelease describes a helm release to be deployed.
pub struct HelmRelease {
    // Name is the name of the Helm release. It accepts environment variables
    // via the go template syntax.
    pub name: String,

    // ChartPath is the local path to a packaged Helm chart or an unpacked Helm
    // chart directory.
    pub chart_path: Option<String>,

    // RemoteChart refers to a remote Helm chart reference or URL.
    pub remote_chart: Option<String>,

    // ValuesFiles are the paths to the Helm `values` files.
    pub values_files: Option<Vec<String>>,

    // Namespace is the Kubernetes namespace.
    pub namespace: Option<String>,

    // Version is the version of the chart.
    pub version: Option<String>,

    // SetValues are key-value pairs. If present, Amphitheatre will send `--set`
    // flag to Helm CLI and append all pairs after the flag.
    pub set_values: Option<HashMap<String, String>>,

    // SetValueTemplates are key-value pairs. If present, Amphitheatre will try
    // to parse the value part of each key-value pair using environment
    // variables in the system, then send `--set` flag to Helm CLI and append
    // all parsed pairs after the flag.
    pub set_value_templates: Option<HashMap<String, String>>,

    // SetFiles are key-value pairs. If present, Amphitheatre will send
    // `--set-file` flag to Helm CLI and append all pairs after the flag.
    pub set_files: Option<HashMap<String, String>>,

    // CreateNamespace if `true`, Amphitheatre will send `--create-namespace`
    // flag to Helm CLI. `--create-namespace` flag is available in Helm since
    // version 3.2. Defaults is `false`.
    pub create_namespace: Option<bool>,

    // Wait if `true`, Amphitheatre will send `--wait` flag to Helm CLI.
    // Defaults to `false`.
    pub wait: Option<bool>,

    // RecreatePods if `true`, Amphitheatre will send `--recreate-pods` flag to
    // Helm CLI when upgrading a new version of a chart in subsequent dev loop
    // deploy. Defaults to `false`.
    pub recreate_pods: Option<bool>,

    // SkipBuildDependencies should build dependencies be skipped. Ignored for
    // `remoteChart`.
    pub skip_build_dependencies: Option<bool>,

    // UseHelmSecrets instructs amphitheatre to use secrets plugin on
    // deployment.
    pub use_helm_secrets: Option<bool>,

    // Repo specifies the helm repository for remote charts. If present,
    // Amphitheatre will send `--repo` Helm CLI flag or flags.
    pub repo: Option<String>,

    // UpgradeOnChange specifies whether to upgrade helm chart on code changes.
    // Default is `true` when helm chart is local (has `chartPath`). Default is
    // `false` when helm chart is remote (has `remoteChart`).
    pub upgrade_on_change: Option<bool>,

    // Overrides are key-value pairs. If present, Amphitheatre will build a Helm
    // `values` file that overrides the original and use it to call Helm CLI
    // (`--f` flag).
    pub overrides: Option<HashMap<String, String>>,

    // Packaged parameters for packaging helm chart (`helm package`).
    pub packaged: Option<HelmPackaged>,
}

/// HelmPackaged parameters for packaging helm chart (`helm package`).
pub struct HelmPackaged {
    // Version sets the `version` on the chart to this semver version.
    pub version: Option<String>,

    // AppVersion sets the `appVersion` on the chart to this version.
    pub app_version: Option<String>,
}

// HelmDeployFlags are additional option flags that are passed on the command
// line to `helm`.
pub struct HelmDeployFlags {
    // Global are additional flags passed on every command.
    pub global: Option<Vec<String>>,

    // Install are additional flags passed to (`helm install`).
    pub install: Option<Vec<String>>,

    // Upgrade are additional flags passed to (`helm upgrade`).
    pub upgrade: Option<Vec<String>>,
}

/// KptDeploy contains all the configuration needed by the deploy steps.
pub struct KptDeploy {
    // Dir is equivalent to the dir in `kpt live apply <dir>`. If not provided,
    // amphitheatre deploys from the default hydrated path
    // `<WORKDIR>/.kpt-pipeline`.
    pub dir: Option<String>,

    // ApplyFlags are additional flags passed to `kpt live apply`.
    pub apply_flags: Option<Vec<String>>,

    // Flags are kpt global flags.
    pub flags: Option<Vec<String>>,

    // Name *alpha* is the inventory object name.
    pub name: Option<String>,

    // InventoryID *alpha* is the inventory ID which annotates the resources
    // being lively applied by kpt.
    pub inventory_id: Option<String>,

    // InventoryNamespace *alpha* sets the inventory namespace.
    pub namespace: Option<String>,

    // Force is used in `kpt live init`, which forces the inventory values to be
    // updated, even if they are already set.
    pub force: Option<bool>,

    // LifecycleHooks describes a set of lifecycle hooks that are executed
    // before and after every deploy.
    pub deploy_hooks: Option<DeployHooks>,

    // DefaultNamespace is the default namespace passed to kpt on deployment if
    // no other override is given.
    pub default_namespace: Option<String>,
}

/// KubectlDeploy *beta* uses a client side `kubectl apply` to deploy manifests.
/// You'll need a `kubectl` CLI version installed that's compatible with your
/// cluster.
pub struct KubectlDeploy {
    // Manifests lists the Kubernetes yaml or json manifests. Defaults to
    // `["k8s/*.yaml"]`. This field is no longer needed in render v2. If given,
    // the v1 kubectl deployer will be triggered.
    pub manifests: Option<Vec<String>>,

    // RemoteManifests lists Kubernetes manifests in remote clusters. This field
    // is only used by v1 kubectl deployer.
    pub remote_manifests: Option<Vec<String>>,

    // Flags are additional flags passed to `kubectl`.
    pub flags: Option<KubectlFlags>,

    // DefaultNamespace is the default namespace passed to kubectl on deployment
    // if no other override is given.
    pub default_namespace: Option<String>,

    // LifecycleHooks describes a set of lifecycle hooks that are executed
    // before and after every deploy.
    pub hooks: Option<DeployHooks>,
}

/// KubectlFlags are additional flags passed on the command line to kubectl
/// either on every command (Global), on creations (Apply) or deletions
/// (Delete).
pub struct KubectlFlags {
    // Global are additional flags passed on every command.
    pub global: Option<Vec<String>>,

    // Apply are additional flags passed on creations (`kubectl apply`).
    pub apply: Option<Vec<String>>,

    // Delete are additional flags passed on deletions (`kubectl delete`).
    pub delete: Option<Vec<String>>,

    // DisableValidation passes the `--validate=false` flag to supported
    // `kubectl` commands when enabled.
    pub disable_validation: Option<bool>,
}

// TODO: KustomizeDeploy shall be deprecated.

/// KustomizeDeploy *beta* uses the `kustomize` CLI to "patch" a deployment for
/// a target environment.
pub struct KustomizeDeploy {
    // KustomizePaths is the path to Kustomization files. Defaults to `["."]`.
    pub paths: Option<Vec<String>>,

    // Flags are additional flags passed to `kubectl`.
    pub flags: Option<KubectlFlags>,

    // BuildArgs are additional args passed to `kustomize build`.
    pub build_args: Option<Vec<String>>,

    // DefaultNamespace is the default namespace passed to kubectl on deployment
    // if no other override is given.
    pub default_namespace: Option<String>,

    // LifecycleHooks describes a set of lifecycle hooks that are executed
    // before and after every deploy.
    pub hooks: Option<DeployHooks>,
}

/// CloudRunDeploy *alpha* deploys the container to Google Cloud Run.
pub struct CloudRunDeploy {
    // ProjectID of the GCP Project to use for Cloud Run.
    pub default_project_id: Option<String>,

    // Region in GCP to use for the Cloud Run Deploy. Must be one of the regions
    // listed in https://cloud.google.com/run/docs/locations.
    pub region: Option<String>,
}

/// LogsConfig configures how container logs are printed as a result of a
/// deployment.
pub struct LogsConfig {
    // Prefix defines the prefix shown on each log line. Valid values are
    // `container`: prefix logs lines with the name of the container.
    // `podAndContainer`: prefix logs lines with the names of the pod and of the
    // container. `auto`: same as `podAndContainer` except that the pod name is
    // skipped if it's the same as the container name. `none`: don't add a
    // prefix. Defaults to `auto`.
    pub prefix: Option<String>,

    // JSONParse defines the rules for parsing/outputting json logs.
    pub json_parse: Option<JSONParseConfig>,
}

/// JSONParseConfig defines the rules for parsing/outputting json logs.
pub struct JSONParseConfig {
    // Fields specifies which top level fields should be printed.
    pub fields: Option<Vec<String>>,
}

type ResourceType = String;

/// PortForwardResource describes a resource to port forward.
#[derive(Default)]
pub struct PortForwardResource {
    // Resource type is the resource type that should be port forward.
    // Acceptable resource types include Kubernetes types: `Service`, `Pod` and
    // Controller resource type that has a pod spec: `ReplicaSet`,
    //  `ReplicationController`, `Deployment`, `StatefulSet`, `DaemonSet`,
    // `Job`, `CronJob`. Standalone `Container` is also valid for Docker
    // deployments.
    pub resource_type: Option<ResourceType>,

    // Name is the name of the Kubernetes resource or local container to port
    // forward.
    pub resource_name: Option<String>,

    // Namespace is the namespace of the resource to port forward. Does not
    // apply to local containers.
    pub namespace: Option<String>,

    // Port is the resource port that will be forward.
    pub port: Option<String>,
}

/// ResourceSelectorConfig contains all the configuration needed by the deploy
/// steps.
#[derive(Default)]
pub struct ResourceSelectorConfig {
    // Allow configures an allowlist for transforming manifests.
    pub allow: Option<Vec<ResourceFilter>>,

    // Deny configures an allowlist for transforming manifests.
    pub deny: Option<Vec<ResourceFilter>>,
}

/// ResourceFilter contains definition to filter which resource to transform.
pub struct ResourceFilter {
    // GroupKind is the compact format of a resource type.
    pub group_kind: String,

    // Image is an optional slice of JSON-path-like paths of where to rewrite
    // images.
    pub image: Vec<String>,

    // Labels is an optional slide of JSON-path-like paths of where to add a
    // labels block if missing.
    pub labels: Vec<String>,
}

/// VerifyTestCase is a list of tests to run on images that Amphitheatre builds.
#[derive(Default)]
pub struct VerifyTestCase {
    // Name is the name descriptor for the verify test.
    pub name: String,

    // Container is the container information for the verify test.
    pub container: Option<Container>,
}

/// @TODO: v1.Container
pub struct Container {}

/// Profile is used to override any `build`, `test` or `deploy` configuration.
#[derive(Default)]
pub struct Profile {
    // Name is a unique profile name. For example: `profile-prod`.
    pub name: String,

    // Activation criteria by which a profile can be auto-activated. The profile
    // is auto-activated if any one of the activations are triggered. An
    // activation is triggered if all of the criteria (env, kubeContext,
    // command) are triggered.
    pub activation: Option<Vec<Activation>>,

    // Patches lists patches applied to the configuration. Patches use the JSON
    // patch notation.
    pub patches: Option<Vec<JSONPatch>>,

    // Pipeline contains the definitions to replace the default Amphitheatre
    // pipeline.
    pub pipeline: Option<Pipeline>,
}

/// Activation criteria by which a profile is auto-activated.
pub struct Activation {
    // Env is a `key=pattern` pair. The profile is auto-activated if an
    // Environment Variable `key` matches the pattern. If the pattern starts
    // with `!`, activation happens if the remaining pattern is _not_ matched.
    // The pattern matches if the Environment Variable value is exactly
    // `pattern`, or the regex `pattern` is found in it. An empty `pattern`
    // (e.g. `env: "key="`) always only matches if the Environment Variable is
    // undefined or empty. For example: `ENV=production`
    pub env: Option<String>,

    // KubeContext is a Kubernetes context for which the profile is
    // auto-activated. For example: `minikube`.
    pub kube_context: Option<String>,

    // Command is a Amphitheatre command for which the profile is
    // auto-activated. For example: `dev`.
    pub command: Option<String>,
}

/// JSONPatch patch to be applied by a profile.
pub struct JSONPatch {
    // Op is the operation carried by the patch: `add`, `remove`, `replace`,
    // `move`, `copy` or `test`. Defaults to `replace`.
    pub op: Option<String>,

    // Path is the position in the yaml where the operation takes place. For
    // example, this targets the `dockerfile` of the first artifact built. For
    // example: `/build/artifacts/0/docker/dockerfile`.
    pub path: String,

    // From is the source position in the yaml, used for `copy` or `move`
    // operations.
    pub from: Option<String>,

    // Value is the value to apply. Can be any portion of yaml.
    pub value: Option<YamlpatchNode>,
}

/// @TODO: util.YamlpatchNode
pub struct YamlpatchNode {}
