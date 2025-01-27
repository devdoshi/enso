use crate::prelude::*;



pub mod project_manager {
    use super::*;
    use std::process::Stdio;

    use ide_ci::define_env_var;

    define_env_var! {
        /// Custom project root. Useful if we want to use backend without affecting user's default
        /// workspace.
        PROJECTS_ROOT, PathBuf;
    }

    #[derive(Clone, Copy, Debug)]
    pub struct ProjectManager;

    impl Program for ProjectManager {
        fn executable_name(&self) -> &'static str {
            "project-manager"
        }
    }

    pub fn spawn_from(bundle: &crate::paths::generated::ProjectManagerBundle) -> Command {
        let binary_path = bundle.bin.project_managerexe.as_path();
        let mut command = <ProjectManager as Program>::Command::new(binary_path);
        // We do this, because Project Manager runs until newline is input. We need to create a pipe
        // to control its lifetime.
        command.stdin(Stdio::piped());
        command
    }
    // #[derive(Shrinkwrap)]
    // #[shrinkwrap(mutable)]
    // pub struct Command(pub ide_ci::program::Command);
    //
    // impl From<ide_ci::program::Command> for Command {
    //     fn from(inner: ide_ci::prelude::Command) -> Self {
    //         Self(inner)
    //     }
    // }
    //
    // impl<P: Program> MyCommand<P> for Command {}
}
