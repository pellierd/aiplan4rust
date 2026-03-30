(define (domain combinatorial-benchmark)
  (:requirements :typing)
  (:types
    location robot - object
  )

  (:predicates
    (at ?r - robot ?l - location)
    (connected ?l1 ?l2 - location)
    (ready ?r - robot)
    (task-completed ?l - location)
  )

  ;; Action 1: Combinatoire pure (2 paramètres)
  ;; Si on a R robots et L locations, on attend R * L instances.
  (:action initialize-robot
    :parameters (?r - robot ?l - location)
    :precondition (and)
    :effect (at ?r ?l)
  )

  ;; Action 2: Propagation en chaîne (Nécessite l'effet de Action 1)
  ;; Si on a R robots, L locations, et C connexions,
  ;; on attend R * C instances (une par robot par connexion valide).
  (:action move
    :parameters (?r - robot ?l1 ?l2 - location)
    :precondition (and (at ?r ?l1) (connected ?l1 ?l2))
    :effect (at ?r ?l2)
  )

  ;; Action 3: Filtrage par constante
  ;; On va mettre un prédicat "ready" uniquement sur un robot spécifique.
  (:action finalize
    :parameters (?r - robot ?l - location)
    :precondition (and (at ?r ?l) (ready ?r))
    :effect (task-completed ?l)
  )
)
