pub(crate) trait Unit {
    type UnitVariant;

    fn unit(self) -> Self::UnitVariant;
}

impl<T> Unit for Option<T> {
    type UnitVariant = Option<()>;

    fn unit(self) -> Self::UnitVariant {
        self.map(|_| ())
    }
}

impl<T, E> Unit for Result<T, E> {
    type UnitVariant = Result<(), E>;

    fn unit(self) -> Self::UnitVariant {
        self.map(|_| ())
    }
}
