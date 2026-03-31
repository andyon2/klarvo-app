use crate::judge::Verdict;

/// A single test result for the report.
pub struct TestResult {
    pub id: String,
    pub name: String,
    pub injection_point: String,
    pub intent: Option<String>,
    pub technique: Option<String>,
    pub evasion: Option<String>,
    pub verdict: Verdict,
    pub output_preview: String,
}

/// Print a summary table of all test results.
pub fn print_summary(results: &[TestResult]) {
    let total = results.len();
    let passed = results.iter().filter(|r| r.verdict.is_pass()).count();
    let failed = results
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Fail(_)))
        .count();
    let inconclusive = results
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Inconclusive(_)))
        .count();

    println!("\n{}", "=".repeat(72));
    println!("  PI SECURITY TEST SUMMARY");
    println!("{}", "=".repeat(72));
    println!("  Total: {total}  |  PASS: {passed}  |  FAIL: {failed}  |  INCONCLUSIVE: {inconclusive}");
    println!("{}", "-".repeat(72));

    for r in results {
        let status = match &r.verdict {
            Verdict::Pass => "\x1b[32mPASS\x1b[0m",
            Verdict::Fail(_) => "\x1b[31mFAIL\x1b[0m",
            Verdict::Inconclusive(_) => "\x1b[33mINCL\x1b[0m",
        };
        println!(
            "  [{:>6}] {:<50} {} | {}",
            r.id, r.name, status, r.injection_point,
        );
        if let Verdict::Fail(detail) = &r.verdict {
            println!("           -> {detail}");
            let preview = truncate(&r.output_preview, 120);
            println!("           -> Output: {preview}");
        }
    }

    println!("{}", "-".repeat(72));

    if failed > 0 {
        println!(
            "\n  \x1b[31m{failed} injection(s) SUCCEEDED — vulnerabilities found!\x1b[0m"
        );
    } else {
        println!("\n  \x1b[32mAll injections were blocked.\x1b[0m");
    }
    println!();
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.replace('\n', "\\n")
    } else {
        format!("{}...", &s[..max].replace('\n', "\\n"))
    }
}
