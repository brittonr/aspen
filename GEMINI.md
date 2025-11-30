# Project Aspen

## Project Overview

Project Aspen is a new core crate designed for the Blixard ecosystem. Its primary goal is to serve as the foundational layer for orchestration, providing essential logic and distributed primitives. It leverages Rust as its primary language and utilizes key technologies such as `iroh` and `hiqlite` for distributed state and communication.

The project is structured into the following main modules:
-   `orchestration`: Handles the high-level orchestration logic.
-   `distributed`: Implements distributed primitives using `iroh` and `hiqlite`.
-   `traits`: Defines core service traits for various components.
-   `types`: Contains the core data structures and types used throughout the system.

## Building and Running

This project is a Rust library crate. You can build and test it using the standard Cargo commands.

-   **Build:**
    ```bash
    cargo build
    ```
-   **Run Tests:**
    ```bash
    cargo nextest run
    ```
-   **Check for compilation errors without building:**
    ```bash
    cargo check
    ```

## Development Conventions

This project adheres to "Tiger Style," a coding philosophy emphasizing safety, performance, and developer experience. The core principles and design goals include:
# Tiger Style

**Version:** 0.1-dev

Tiger Style is a coding philosophy focused on safety, performance, and developer experience. Inspired by the practices of TigerBeetle, it focuses on building robust, efficient, and maintainable software through disciplined engineering.

## 1. Core principles

Tiger Style is not just a set of coding standards; it's a practical approach to software development. By prioritizing safety, performance, and developer experience, you create code that is reliable, efficient, and enjoyable to work with.

### Safety
Safety is the foundation of Tiger Style. It means writing code that works in all situations and reduces the risk of errors. Focusing on safety makes your software reliable and trustworthy.

### Performance
Performance is about using resources efficiently to deliver fast, responsive software. Prioritizing performance early helps you design systems that meet or exceed user expectations.

### Developer experience
A good developer experience improves code quality and maintainability. Readable and easy-to-work-with code encourages collaboration and reduces errors, leading to a healthier codebase that stands the test of time.

---

## 2. Design goals

The design goals focus on building software that is safe, fast, and easy to maintain.

### 2.1. Safety

Safety in coding relies on clear, structured practices that prevent errors and strengthen the codebase. It's about writing code that works in all situations and catches problems early. By focusing on safety, you create reliable software that behaves predictably no matter where it runs.

#### Control and limits
Predictable control flow and bounded system resources are essential for safe execution.

* **Simple and explicit control flow:** Favor straightforward control structures over complex logic. Simple control flow makes code easier to understand and reduces the risk of bugs. Avoid recursion if possible to keep execution bounded and predictable, preventing stack overflows and uncontrolled resource use.
* **Set fixed limits:** Set explicit upper bounds on loops, queues, and other data structures. Fixed limits prevent infinite loops and uncontrolled resource use, following the fail-fast principle. This approach helps catch issues early and keeps the system stable.
* **Limit function length:** Keep functions concise, ideally under 70 lines. Shorter functions are easier to understand, test, and debug. They promote single responsibility, where each function does one thing well, leading to a more modular and maintainable codebase.
* **Centralize control flow:** Keep switch or if statements in the main parent function, and move non-branching logic to helper functions. Let the parent function manage state, using helpers to calculate changes without directly applying them. Keep leaf functions pure and focused on specific computations. This divides responsibility: one function controls flow, others handle specific logic.

#### Memory and types
Clear and consistent handling of memory and types is key to writing safe, portable code.

* **Use explicitly sized types:** Use data types with explicit sizes, like `u32` or `i64`, instead of architecture-dependent types like `usize`. This keeps behavior consistent across platforms and avoids size-related errors, improving portability and reliability.
* **Static memory allocation:** Allocate all necessary memory during startup and avoid dynamic memory allocation after initialization. Dynamic allocation at runtime can cause unpredictable behavior, fragmentation, and memory leaks. Static allocation makes memory management simpler and more predictable.
* **Minimize variable scope:** Declare variables in the smallest possible scope. Limiting scope reduces the risk of unintended interactions and misuse. It also makes the code more readable and easier to maintain by keeping variables within their relevant context.

#### Error handling
Correct error handling keeps the system robust and reliable in all conditions.

* **Use assertions:** Use assertions to verify that conditions hold true at specific points in the code. Assertions work as internal checks, increase robustness, and simplify debugging.
* **Assert function arguments and return values:** Check that functions receive and return expected values.
* **Validate invariants:** Keep critical conditions stable by asserting invariants during execution.
* **Use pair assertions:** Check critical data at multiple points to catch inconsistencies early.
* **Fail fast on programmer errors:** Detect unexpected conditions immediately, stopping faulty code from continuing.
* **Handle all errors:** Check and handle every error. Ignoring errors can lead to undefined behavior, security issues, or crashes. Write thorough tests for error-handling code to make sure your application works correctly in all cases.
* **Treat compiler warnings as errors:** Use the strictest compiler settings and treat all warnings as errors. Warnings often point to potential issues that could cause bugs. Fixing them right away improves code quality and reliability.
*   **Avoid implicit defaults:** Explicitly specify options when calling library functions instead of relying on defaults. Implicit defaults can change between library versions or across environments, causing inconsistent behavior. Being explicit improves code clarity and stability.
*   **Use property-based testing:** Beyond unit tests, use property-based testing (e.g., `proptest`) to explore edge cases and ensure invariants hold across a wide range of inputs. This increases confidence in correctness and reduces the likelihood of subtle bugs.

### 2.2. Performance

Performance is about using resources efficiently to deliver fast, responsive software. Prioritizing performance early helps design systems that meet or exceed user expectations without unnecessary overhead.

#### Design for performance
Early design decisions have a significant impact on performance. Thoughtful planning helps avoid bottlenecks later.

* **Design for performance early:** Consider performance during the initial design phase. Early architectural decisions have a big impact on overall performance, and planning ahead ensures you can avoid bottlenecks and improve resource efficiency.
* **Napkin math:** Use quick, back-of-the-envelope calculations to estimate system performance and resource costs. For example, estimate how long it takes to read 1 GB of data from memory or what the expected storage cost will be for logging 100,000 requests per second. This helps set practical expectations early and identify potential bottlenecks before they occur.
* **Batch operations:** Amortize expensive operations by processing multiple items together. Batching reduces overhead per item, increases throughput, and is especially useful for I/O-bound operations.

#### Efficient resource use
Focus on optimizing the slowest resources, typically in this order:

1.  **Network:** Optimize data transfer and reduce latency.
2.  **Disk:** Improve I/O operations and manage storage efficiently.
3.  **Memory:** Use memory effectively to prevent leaks and overuse.
4.  **CPU:** Increase computational efficiency and reduce processing time.

#### Predictability
Writing predictable code improves performance by reducing CPU cache misses and optimizing branch prediction.

* **Ensure predictability:** Write code with predictable execution paths. Predictable code uses CPU caching and branch prediction better, leading to improved performance. Avoid patterns that cause frequent cache misses or unpredictable branching, as they degrade performance.
* **Reduce compiler dependence:** Don't rely solely on compiler optimizations for performance. Write clear, efficient code that doesn't depend on compiler behavior. Be explicit in performance-critical sections to ensure consistent results across compilers.

### 2.3. Developer experience

Improving the developer experience creates a more maintainable and collaborative codebase.

#### Name things
Get the nouns and verbs right. Great names capture what something is or does and create a clear, intuitive model. They show you understand the domain. Take time to find good names, where nouns and verbs fit together, making the whole greater than the sum of its parts.

* **Clear and consistent naming:** Use descriptive and meaningful names for variables, functions, and files. Good naming improves code readability and helps others understand each component's purpose. Stick to a consistent style, like `snake_case`, throughout the codebase.
* **Avoid abbreviations:** Use full words in names unless the abbreviation is widely accepted and clear (e.g., `ID`, `URL`). Abbreviations can be confusing and make it harder for others, especially new contributors, to understand the code.
* **Include units or qualifiers in names:** Append units or qualifiers to variable names, placing them in descending order of significance (e.g., `latency_ms_max` instead of `max_latency_ms`). This clears up meaning, avoids confusion, and ensures related variables, like `latency_ms_min`, line up logically and group together.
* **Document the 'why':** Use comments to explain why decisions were made, not just what the code does. Knowing the intent helps others maintain and extend the code properly. Give context for complex algorithms, unusual approaches, or key constraints.
* **Use proper comment style:** Write comments as complete sentences with correct punctuation and grammar. Clear, professional comments improve readability and show attention to detail. They help create a cleaner, more maintainable codebase.

#### Organize things
Organizing code well makes it easy to navigate, maintain, and extend. A logical structure reduces cognitive load, letting developers focus on solving problems instead of figuring out the code. Group related elements, and simplify interfaces to keep the codebase clean, scalable, and manageable as complexity grows.

* **Organize code logically:** Structure your code logically. Group related functions and classes together. Order code naturally, placing high-level abstractions before low-level details. Logical organization makes code easier to navigate and understand.
* **Simplify function signatures:** Keep function interfaces simple. Limit parameters, and prefer returning simple types. Simple interfaces reduce cognitive load, making functions easier to understand and use correctly.
* **Construct objects in-place:** Initialize large structures or objects directly where they are declared. In-place construction avoids unnecessary copying or moving of data, improving performance and reducing the potential for lifecycle errors.
* **Minimize variable scope:** Declare variables close to their usage and within the smallest necessary scope. This reduces the risk of misuse and makes code easier to read and maintain.

#### Ensure consistency
Maintaining consistency in your code helps reduce errors and creates a stable foundation for the rest of the system.

* **Avoid duplicates and aliases:** Prevent inconsistencies by avoiding duplicated variables or unnecessary aliases. When two variables represent the same data, there's a higher chance they fall out of sync. Use references or pointers to maintain a single source of truth.
* **Pass large objects by reference:** If a function's argument is larger than 16 bytes, pass it as a reference instead of by value to avoid unnecessary copying. This can catch bugs early where unintended copies may occur.
* **Minimize dimensionality:** Keep function signatures and return types simple to reduce the number of cases a developer has to handle. For example, prefer `void` over `bool`, `bool` over `u64`, and so on, when it suits the function's purpose.
* **Handle buffer allocation cleanly:** When working with buffers, allocate them close to where they are used and ensure all corresponding cleanup happens in the same logical block. Group resource allocation and deallocation with clear newlines to make leaks easier to identify.

#### Avoid off-by-one errors
Off-by-one errors often result from casual interactions between an `index`, a `count`, or a `size`. Treat these as distinct types, and apply clear rules when converting between them.

* **Indexes, counts, and sizes:** Indexes are 0-based, counts are 1-based, and sizes represent total memory usage. When converting between them, add or multiply accordingly. Use meaningful names with units or qualifiers to avoid confusion.
* **Handle division intentionally:** When dividing, make your intent clear by specifying how rounding should be handled in edge cases. Use functions or operators designed for exact division, floor division, or ceiling division. This avoids ambiguity and ensures the result behaves as expected.

#### Code consistency and tooling
Consistency in code style and tools improves readability, reduces mental load, and makes working together easier.

* **Maintain consistent indentation:** Use a uniform indentation style across the codebase. For example, using 4 spaces for indentation provides better visual clarity, especially in complex structures.
* **Limit line lengths:** Keep lines within a reasonable length (e.g., 100 characters) to ensure readability. This prevents horizontal scrolling and helps maintain an accessible code layout.
* **Use clear code blocks:** Structure code clearly by separating blocks (e.g., control structures, loops, function definitions) to make it easy to follow. Avoid placing multiple statements on a single line, even if allowed. Consistent block structures prevent subtle logic errors and make code easier to maintain.
* **Minimize external dependencies:** Reducing external dependencies simplifies the build process and improves security management. Fewer dependencies lower the risk of supply chain attacks, minimize performance issues, and speed up installation.
* **Standardize tooling:** Using a small, standardized set of tools simplifies the development environment and reduces accidental complexity. Choose cross-platform tools where possible to avoid platform-specific issues and improve portability across systems.

---

## Addendum: Zero technical debt

While Tiger Style focuses on the core principles of safety, performance, and developer experience, these are reinforced by an underlying commitment to zero technical debt.

A zero technical debt policy is key to maintaining a healthy codebase and ensuring long-term productivity. Addressing potential issues proactively and building robust solutions from the start helps avoid debt that would slow future development.

* **Do it right the first time:** Take the time to design and implement solutions correctly from the start. Rushed features lead to technical debt that requires costly refactoring later.
* **Be proactive in problem-solving:** Anticipate potential issues and fix them before they escalate. Early detection saves time and resources, preventing performance bottlenecks and architectural flaws.
* **Build momentum:** Delivering solid, reliable code builds confidence and enables faster development cycles. High-quality work supports innovation and reduces the need for future rewrites.

Avoiding technical debt ensures that progress is true progress—solid, reliable, and built to last.

---

## Addendum: Performance estimation

You should think about performance early in design. Napkin math is a helpful tool for this.

Napkin math uses simple calculations and rounded numbers to quickly estimate system performance and resource needs.

* **Quick insights:** Understand system behavior fast without deep analysis.
* **Early decisions:** Find potential bottlenecks early in design.
* **Sanity checks:** See if an idea works before you build it.

For example, if you're designing a system to store logs, you can estimate storage costs like this:

1.  **Estimate log volume:** Assume 1,000 requests per second (RPS). Each log entry is about 1 KB.
2.  **Calculate daily log volume:**
    `1,000 RPS * 86,400 seconds/day * 1 KB ≈ 86,400,000 KB/day ≈ 86.4 GB/day`
3.  **Estimate monthly storage:**
    `86.4 GB/day * 30 days ≈ 2,592 GB/month`
4.  **Estimate cost** (using $0.02 per GB for blob storage):
    `2,592 GB * 1000 GB/TB * $0.02/GB ≈ $51 per month`

This gives you a rough idea of monthly storage costs. It helps you check if your logging plan works. The idea is to get within 10x of the right answer.

For more, see Simon Eskildsen's napkin math project.

---

## Colophon

This document is a "remix" inspired by the original **Tiger Style** guide from the TigerBeetle project. In the spirit of Remix Culture, parts of this document are verbatim copies of the original work, while other sections have been rewritten or adapted to fit the goals of this version. This remix builds upon the principles outlined in the original document with a more general approach.

* **Maintained by:** Simon Klee
* **Version:** 0.1-dev
* **Last updated:** October 2024
* **License:** CC BY 4.0
* **Source:** [github.com/simonklee/tigerstyle](https://github.com/simonklee/tigerstyle)

