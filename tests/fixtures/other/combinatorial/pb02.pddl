(define (problem pb02-stress)
  (:domain combinatorial-benchmark)
  (:objects
    r1 r2 r3 r4 r5 r6 r7 r8 r9 r10 - robot
    l1 l2 l3 l4 l5 l6 l7 l8 l9 l10 l11 l12 l13 l14 l15 l16 l17 l18 l19 l20 - location
  )

  (:init
    ;; Un seul robot est prêt pour l'action finale
    (ready r1)

    ;; On connecte chaque location i à i+1 et i-1 (Ligne)
    ;; PLUS des connexions "Sauts" pour augmenter la combinatoire
    (connected l1 l2) (connected l2 l1) (connected l2 l3) (connected l3 l2)
    (connected l3 l4) (connected l4 l3) (connected l4 l5) (connected l5 l4)
    (connected l5 l6) (connected l6 l5) (connected l6 l7) (connected l7 l6)
    (connected l7 l8) (connected l8 l7) (connected l8 l9) (connected l9 l8)
    (connected l9 l10) (connected l10 l9) (connected l10 l11) (connected l11 l10)
    (connected l11 l12) (connected l12 l11) (connected l12 l13) (connected l13 l12)
    (connected l13 l14) (connected l14 l13) (connected l14 l15) (connected l15 l14)
    (connected l15 l16) (connected l16 l15) (connected l16 l17) (connected l17 l16)
    (connected l17 l18) (connected l18 l17) (connected l18 l19) (connected l19 l18)
    (connected l19 l20) (connected l20 l19)

    ;; Ajout de connexions transversales (Star Topology au centre)
    ;; Cela multiplie les chemins possibles pour le Datalog
    (connected l1 l10) (connected l10 l1)
    (connected l5 l15) (connected l15 l5)
    (connected l2 l20) (connected l20 l2)
    (connected l8 l12) (connected l12 l8)
  )

  (:goal (and (task-completed l20)))
)
