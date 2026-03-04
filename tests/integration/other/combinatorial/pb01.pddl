(define (problem prob-count-fixed)
  (:domain combinatorial-benchmark)
  (:objects
    r1 r2 - robot
    loc1 loc2 loc3 - location
  )

  (:init
    ;; Connexions (3 faits)
    (connected loc1 loc2)
    (connected loc2 loc3)
    (connected loc3 loc1)

    ;; Un seul robot est prêt
    (ready r1)
  )
  ;; AJOUTE CECI :
    (:goal (and))
)
