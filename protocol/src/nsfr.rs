mod numbers;
use miette::Diagnostic;
pub use numbers::*;

#[inline(always)]
pub fn when_the_miette<T, E: Diagnostic>(result: Result<T, E>) -> Result<T, E> {
    match result {
        Ok(_) => result,
        Err(e) => {
            use miette::ReportHandler;

            let mut buf = String::new();

            miette::MietteHandlerOpts::new()
                .build()
                .debug(&e, &mut std::fmt::Formatter::new(&mut buf))
                .expect("miette failed... sadge");

            eprintln!("{buf}");
            Err(e)
        }
    }
}
