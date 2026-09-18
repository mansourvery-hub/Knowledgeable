use application::graph_service::GraphService;
use domain::{ConceptNode, ConceptStatus};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = SqlitePool::connect("sqlite://knowledgeable.db").await?;
    let graph = GraphService::new(Arc::new(pool));

    let concepts = vec![
        (
            "Prime Number",
            "A natural number greater than 1 that is not a product of two smaller natural numbers.",
        ),
        ("Composite Number", "A positive integer greater than 1 that is not prime."),
        ("Factor", "A number that divides another number evenly."),
        ("Multiple", "A number that is the product of a given number and an integer."),
        ("Divisibility", "The property of being divisible by another number without a remainder."),
        ("Integer", "A whole number, positive, negative, or zero."),
        ("Natural Number", "A positive integer (1, 2, 3...)."),
        (
            "Fundamental Theorem of Arithmetic",
            "Every integer > 1 is either prime or a unique product of primes.",
        ),
        (
            "Sieve of Eratosthenes",
            "An ancient algorithm for finding all prime numbers up to any given limit.",
        ),
        ("Even Number", "An integer divisible by 2."),
        ("Odd Number", "An integer not divisible by 2."),
        (
            "Greatest Common Divisor",
            "The largest positive integer that divides each of two or more integers.",
        ),
        (
            "Least Common Multiple",
            "The smallest positive integer that is divisible by each of two or more integers.",
        ),
        ("Perfect Number", "A positive integer that is equal to the sum of its proper divisors."),
        (
            "Abundant Number",
            "A number for which the sum of its proper divisors is greater than the number.",
        ),
        (
            "Deficient Number",
            "A number for which the sum of its proper divisors is less than the number.",
        ),
        ("Mersenne Prime", "A prime number that is one less than a power of two."),
        ("Twin Primes", "A pair of prime numbers that differ by two."),
        ("Goldbach's Conjecture", "Every even integer greater than 2 is the sum of two primes."),
        (
            "Prime Factorization",
            "Finding the prime numbers that multiply together to make the original number.",
        ),
    ];

    for (name, statement) in concepts {
        let node = ConceptNode {
            id: Uuid::new_v4(),
            canonical_name: name.into(),
            canonical_statement: statement.into(),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        graph.create_concept(&node).await?;
        println!("Seeded concept: {}", name);
    }
    Ok(())
}
