use pinocchio::program_error::ProgramError;


pub enum PinocchioError {
    NotSigner,
    InvalidOwner,
    InvalidAccountData,
    InvalidAddress,
}

impl From<PinocchioError> for ProgramError {
    fn from(e: PinocchioError) -> Self {
        match e {
            PinocchioError::NotSigner => ProgramError::MissingRequiredSignature,
            PinocchioError::InvalidOwner => ProgramError::IllegalOwner,
            PinocchioError::InvalidAccountData => ProgramError::InvalidAccountData,
            PinocchioError::InvalidAddress => ProgramError::InvalidSeeds,
        }
    }
}