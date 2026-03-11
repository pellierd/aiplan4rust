(define (problem pb-limit)
  (:domain combinatorial-benchmark)
  (:objects
    r1 r2 r3 r4 r5 r6 r7 r8 r9 r10 r11 r12 r13 r14 r15 r16 r17 r18 r19 r20 r21 r22 r23 r24 r25 r26 r27 r28 r29 r30 r31 r32 r33 r34 r35 r36 r37 r38 r39 r40 r41 r42 r43 r44 r45 r46 r47 r48 r49 r50 - robot
    l1 l2 l3 l4 l5 l6 l7 l8 l9 l10 h1 h2 h3 h4 h5 h6 h7 h8 h9 h10 s1 s2 s3 s4 s5 s6 s7 s8 s9 s10 s11 s12 s13 s14 s15 s16 s17 s18 s19 s20 s21 s22 s23 s24 s25 s26 s27 s28 s29 s30 - location
  )

  (:init
    (ready r1)

    ;; --- STRUCTURE HUB-TO-HUB ---
    ;; On connecte les 10 hubs (h1..h10) entre eux (Clique de 10 = 90 connexions)
    (connected h1 h2) (connected h1 h3) (connected h1 h4) (connected h1 h5) (connected h1 h6) (connected h1 h7) (connected h1 h8) (connected h1 h9) (connected h1 h10)
    (connected h2 h1) (connected h2 h3) (connected h2 h4) (connected h2 h5) (connected h2 h6) (connected h2 h7) (connected h2 h8) (connected h2 h9) (connected h2 h10)
    (connected h3 h1) (connected h3 h2) (connected h3 h4) (connected h3 h5) (connected h3 h6) (connected h3 h7) (connected h3 h8) (connected h3 h9) (connected h3 h10)
    (connected h4 h1) (connected h4 h2) (connected h4 h3) (connected h4 h5) (connected h4 h6) (connected h4 h7) (connected h4 h8) (connected h4 h9) (connected h4 h10)
    (connected h5 h1) (connected h5 h2) (connected h5 h3) (connected h5 h4) (connected h5 h6) (connected h5 h7) (connected h5 h8) (connected h5 h9) (connected h5 h10)
    (connected h6 h1) (connected h6 h2) (connected h6 h3) (connected h6 h4) (connected h6 h5) (connected h6 h7) (connected h6 h8) (connected h6 h9) (connected h6 h10)
    (connected h7 h1) (connected h7 h2) (connected h7 h3) (connected h7 h4) (connected h7 h5) (connected h7 h6) (connected h7 h8) (connected h7 h9) (connected h7 h10)
    (connected h8 h1) (connected h8 h2) (connected h8 h3) (connected h8 h4) (connected h8 h5) (connected h8 h6) (connected h8 h7) (connected h8 h9) (connected h8 h10)
    (connected h9 h1) (connected h9 h2) (connected h9 h3) (connected h9 h4) (connected h9 h5) (connected h9 h6) (connected h9 h7) (connected h9 h8) (connected h9 h10)
    (connected h10 h1) (connected h10 h2) (connected h10 h3) (connected h10 h4) (connected h10 h5) (connected h10 h6) (connected h10 h7) (connected h10 h8) (connected h10 h9)

    ;; --- STRUCTURE HUB-TO-SATELLITES ---
    ;; On connecte chaque satellite (s1..s30) à DEUX hubs au hasard
    ;; Cela crée des milliers de chemins possibles pour chaque robot
    (connected s1 h1) (connected s1 h2) (connected s2 h2) (connected s2 h3) (connected s3 h3) (connected s3 h4) (connected s4 h4) (connected s4 h5) (connected s5 h5) (connected s5 h6)
    (connected s6 h6) (connected s6 h7) (connected s7 h7) (connected s7 h8) (connected s8 h8) (connected s8 h9) (connected s9 h9) (connected s9 h10) (connected s10 h10) (connected s10 h1)
    (connected s11 h1) (connected s11 h3) (connected s12 h2) (connected s12 h4) (connected s13 h3) (connected s13 h5) (connected s14 h4) (connected s14 h6) (connected s15 h5) (connected s15 h7)
    (connected s16 h6) (connected s16 h8) (connected s17 h7) (connected s17 h9) (connected s18 h8) (connected s18 h10) (connected s19 h9) (connected s19 h1) (connected s20 h10) (connected s20 h2)
    (connected s21 h1) (connected s21 h4) (connected s22 h2) (connected s22 h5) (connected s23 h3) (connected s23 h6) (connected s24 h4) (connected s24 h7) (connected s25 h5) (connected s25 h8)
    (connected s26 h6) (connected s26 h9) (connected s27 h7) (connected s27 h10) (connected s28 h8) (connected s28 h1) (connected s29 h9) (connected s29 h2) (connected s30 h10) (connected s30 h3)

    ;; Et le retour (bi-directionnel)
    (connected h1 s1) (connected h2 s1) (connected h2 s2) (connected h3 s2) (connected h3 s3) (connected h4 s3) (connected h4 s4) (connected h5 s4) (connected h5 s5) (connected h6 s5)
    (connected h6 s6) (connected h7 s6) (connected h7 s7) (connected h8 s7) (connected h8 s8) (connected h9 s8) (connected h9 s9) (connected h10 s9) (connected h10 s10) (connected h1 s10)
    ;; (Tu peux arrêter ici, le nombre d'actions sera déjà colossal)
  )

  (:goal (and (task-completed s30)))
)
