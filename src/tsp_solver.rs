use rand::seq::SliceRandom;
use rand::thread_rng;

use misc::Coords;

use crate::defn::{Defn, Cell};
use crate::solver::{Outcome, Findings, Constraints, Progress};
use crate::env::Env;

use rand::Rng;
use std::collections::{BTreeMap, BTreeSet};

/// Ein Individuum stellt eine mögliche Reihenfolge von Koordinaten dar
#[derive(Clone, Debug)]
pub struct TspIndividual {
    pub order: Vec<Coords>,
    pub fitness: Option<u32>, // Anzahl Schritte zur Lösung, wird später gesetzt
}

impl TspIndividual {
    pub fn new(order: Vec<Coords>) -> Self {
        Self {
            order,
            fitness: None,
        }
    }
}

/// Erzeugt eine initiale Population mit zufälligen Permutationen der Zellen
pub fn generate_initial_population(
    defn: &Defn,
    population_size: usize,
) -> Vec<TspIndividual> {
    // Wähle nur die unaufgedeckten, lösbaren Zellen aus
    let mut solvable_cells: Vec<Coords> = defn
        .iter()
        .filter_map(|(coords, cell)| {
            match cell {
                Cell::Zone0 { revealed: false, .. }
                | Cell::Zone6 { revealed: false, .. }
                | Cell::Zone18 { revealed: false } => Some(*coords),
                _ => None,
            }
        })
        .collect();

    let mut population = Vec::with_capacity(population_size);
    for _ in 0..population_size {
        let mut rng = thread_rng();
        solvable_cells.shuffle(&mut rng);
        population.push(TspIndividual::new(solvable_cells.clone()));
    }

    population
}



/// Bewertet ein Individuum – je weniger Schritte, desto besser.
/// Gibt `None` zurück, wenn der Lösungsversuch scheitert (z. B. Reihenfolge unbrauchbar).
pub fn evaluate_fitness(
    individual: &mut TspIndividual,
    defn: &Defn,
    env: &mut Env,
) -> Option<u32> {
    let mut progress = Progress::of_defn(defn);
    let mut constraints = Constraints::of_defn(defn);
    let mut steps = 0;
    let max_steps = 500;

    let mut last_unknowns = progress.unknown_count();
    let mut attempts = 0;

    println!("FITNESS: evaluate_fitness gestartet für Individuum");

    while !progress.is_empty() && attempts < max_steps {
        println!(
            "-> Step {}: {} Zellen noch ungelöst",
            steps,
            progress.unknown_count()
        );
        attempts += 1;

        let visible_cells: BTreeSet<_> = progress.blacks().union(progress.blues()).cloned().collect();
        constraints.reveal(&visible_cells);
        constraints.narrow(&visible_cells, &progress);
        constraints.gc();

        let mut updated = false;

        for coords in &individual.order {
            if progress.is_known(coords) {
                continue;
            }

            // Versuche: triviale Invarianten
            let mut invariants = constraints.trivial_invariants(defn);

            // Wenn keine trivialen -> versuche compound
            if invariants.is_empty() {
                env.reset_timer();
                if let Ok((compound, _)) = constraints.compound_invariants(env, defn) {
                    invariants = compound;
                }
            }

            // Wenn keine compound -> versuche globale Invarianten
            if invariants.is_empty() {
                if let Ok(global_invariants) = constraints.global_invariants(env, defn) {
                    invariants = global_invariants;
                }
            }

            // Falls etwas gefunden -> anwenden
            if let Some(color) = invariants.get(coords) {
                progress.update(BTreeMap::from([(*coords, *color)]));
                steps += 1;
                updated = true;
                break;
            }
        }

        if !updated {
            break;
        }

        let current_unknowns = progress.unknown_count();
        if current_unknowns == last_unknowns {
            break;
        }
        last_unknowns = current_unknowns;
    }

    if progress.is_solved() {
        println!("GEFUNDEN: Lösung in {} Schritten gefunden", steps);
        individual.fitness = Some(steps);
        return Some(steps);
    }

    println!("INDIVID NOT SOLVEABLE: Individuum konnte nicht gelöst werden.");
    individual.fitness = None;
    None
}



/// Wählt ein Individuum mit der besten Fitness aus `k` zufälligen Kandidaten.
/// Gibt `None` zurück, wenn keine Fitness vorhanden ist (z. B. bei ungültiger Lösung).
pub fn select_parent(population: &[TspIndividual], k: usize) -> Option<&TspIndividual> {
    let mut rng = thread_rng();
    let candidates: Vec<_> = population
        .choose_multiple(&mut rng, k)
        .filter(|ind| ind.fitness.is_some())
        .collect();

    candidates.into_iter().min_by_key(|ind| ind.fitness.unwrap())
}

/// Führt Order Crossover (OX) zwischen zwei Eltern durch und erzeugt ein Kind.
/// Die Reihenfolge bleibt eine gültige Permutation.
pub fn crossover(parent1: &TspIndividual, parent2: &TspIndividual) -> TspIndividual {
    let len = parent1.order.len();
    let mut rng = rand::thread_rng();

    // Zufälliger Abschnitt (start..=end) von parent1
    let (start, end) = {
        let i = rng.gen_range(0..len);
        let j = rng.gen_range(0..len);
        if i < j { (i, j) } else { (j, i) }
    };

    //Abschnitt von Parent 1 kopieren
    let mut child_order: Vec<Option<Coords>> = vec![None; len];
    for i in start..=end {
        child_order[i] = Some(parent1.order[i]);
    }

    //Fehlende Werte aus Parent 2 vorbereiten
    let missing_values: Vec<Coords> = parent2
        .order
        .iter()
        .filter(|c| !child_order.contains(&Some(**c)))
        .cloned()
        .collect();

    //Rest auffüllen
    let mut iter = missing_values.into_iter();
    for i in 0..len {
        if child_order[i].is_none() {
            child_order[i] = Some(iter.next().expect("Fehlender Wert bei Crossover"));
        }
    }

    // Final: unwrap() der Option<Coords> -> garantiert safe
    let final_order = child_order.into_iter().map(|c| c.unwrap()).collect();

    TspIndividual::new(final_order)
}

/// Mutiert ein Individuum mit gegebener Wahrscheinlichkeit.
/// Swap-Mutation: Tausche zwei zufällige Zellen.
pub fn mutate(individual: &mut TspIndividual, mutation_rate: f64) {
    let mut rng = thread_rng();
    if rng.gen::<f64>() < mutation_rate {
        let len = individual.order.len();
        if len < 2 {
            return;
        }

        let i = rng.gen_range(0..len);
        let mut j = rng.gen_range(0..len);
        while j == i {
            j = rng.gen_range(0..len);
        }

        individual.order.swap(i, j);
    }
}


/// Führt den genetischen Algorithmus über mehrere Generationen aus.
/// Gibt das beste gefundene Individuum zurück.
pub fn evolve(
    defn: &Defn,
    env: &mut Env,
    population_size: usize,
    generations: usize,
    tournament_k: usize,
    mutation_rate: f64,
    elitism: usize,
) -> Option<TspIndividual> {
    // Initiale Population erzeugen und bewerten
    let mut population = generate_initial_population(defn, population_size);
    for individual in &mut population {
        evaluate_fitness(individual, defn, env);
    }

    for gen in 0..generations {
        println!("GENERATION STARTED: Generation {} gestartet...", gen);

        let mut next_gen = Vec::new();

        //  Elitismus – beste Individuen behalten
        population.sort_by_key(|ind| ind.fitness.unwrap_or(u32::MAX));
        next_gen.extend_from_slice(&population[..elitism]);

        //  Eltern + Crossover + Mutation
        while next_gen.len() < population_size {
            let parent1 = select_parent(&population, tournament_k)?;
            let parent2 = select_parent(&population, tournament_k)?;

            let mut child = crossover(parent1, parent2);
            mutate(&mut child, mutation_rate);
            evaluate_fitness(&mut child, defn, env);
            next_gen.push(child);
        }

        population = next_gen;
    }

    // Bestes Ergebnis zurückgeben
    population.into_iter().min_by_key(|ind| ind.fitness.unwrap_or(u32::MAX))
}


/// führt den TSP_Solver aus
pub fn run(env: &mut Env, defn: &Defn, verbose: bool) -> Outcome {
    let population_size = 50;
    let generations = 100;
    let tournament_k = 5;
    let mutation_rate = 0.1;
    let elitism = 2;

    if verbose {
        println!("RUNNING: TSP-Solver läuft...");
        println!(
            "-> Population: {}, Generationen: {}, Mutation: {:.2}, Elitismus: {}",
            population_size, generations, mutation_rate, elitism
        );
    }

    let best = evolve(
        defn,
        env,
        population_size,
        generations,
        tournament_k,
        mutation_rate,
        elitism,
    );

    match best {
        Some(individual) => {
            let steps = individual.fitness.unwrap_or(u32::MAX);
            println!("FOUND: Beste Lösung gefunden mit {} Schritten.", steps);
    
            if verbose {
                println!("ORDER: Besuchsreihenfolge der Zellen:");
                for (i, coords) in individual.order.iter().enumerate() {
                    println!("  {:2}. {:?}", i + 1, coords);
                }
            }
    
            let findings_vec: Vec<Findings> = individual
                .order
                .iter()
                .map(|coords| Findings::new_local(*coords))
                .collect();
            Outcome::Solved(findings_vec)
        }
        None => {
            println!("ERROR: Keine gültige Lösung gefunden.");
            Outcome::Unsolvable
        }
    }
}