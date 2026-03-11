(define (problem pb05-mega)
  (:domain combinatorial-benchmark)
  (:objects
    r1 r2 r3 r4 r5 r6 r7 r8 r9 r10 r11 r12 r13 r14 r15 r16 r17 r18 r19 r20 r21 r22 r23 r24 r25 r26 r27 r28 r29 r30 r31 r32 r33 r34 r35 r36 r37 r38 r39 r40 r41 r42 r43 r44 r45 r46 r47 r48 r49 r50 r51 r52 r53 r54 r55 r56 r57 r58 r59 r60 r61 r62 r63 r64 r65 r66 r67 r68 r69 r70 r71 r72 r73 r74 r75 r76 r77 r78 r79 r80 - robot
    h1 h2 h3 h4 h5 h6 h7 h8 h9 h10 h11 h12 h13 h14 h15 h16 h17 h18 h19 h20 s1 s2 s3 s4 s5 s6 s7 s8 s9 s10 s11 s12 s13 s14 s15 s16 s17 s18 s19 s20 s21 s22 s23 s24 s25 s26 s27 s28 s29 s30 s31 s32 s33 s34 s35 s36 s37 s38 s39 s40 s41 s42 s43 s44 s45 s46 s47 s48 s49 s50 s51 s52 s53 s54 s55 s56 s57 s58 s59 s60 - location
  )

  (:init
    (ready r1)

    ;; --- CLIQUE DES HUBS (20 hubs connectés entre eux = 380 paires) ---
    ;; Copie ce bloc pour créer le maillage central
    (connected h1 h2) (connected h1 h3) (connected h1 h4) (connected h1 h5) (connected h1 h6) (connected h1 h7) (connected h1 h8) (connected h1 h9) (connected h1 h10) (connected h1 h11) (connected h1 h12) (connected h1 h13) (connected h1 h14) (connected h1 h15) (connected h1 h16) (connected h1 h17) (connected h1 h18) (connected h1 h19) (connected h1 h20)
    (connected h2 h1) (connected h2 h3) (connected h2 h4) (connected h2 h5) (connected h2 h6) (connected h2 h7) (connected h2 h8) (connected h2 h9) (connected h2 h10) (connected h2 h11) (connected h2 h12) (connected h2 h13) (connected h2 h14) (connected h2 h15) (connected h2 h16) (connected h2 h17) (connected h2 h18) (connected h2 h19) (connected h2 h20)
    (connected h3 h1) (connected h3 h2) (connected h3 h4) (connected h3 h5) (connected h3 h6) (connected h3 h7) (connected h3 h8) (connected h3 h9) (connected h3 h10) (connected h3 h11) (connected h3 h12) (connected h3 h13) (connected h3 h14) (connected h3 h15) (connected h3 h16) (connected h3 h17) (connected h3 h18) (connected h3 h19) (connected h3 h20)
    (connected h4 h1) (connected h4 h2) (connected h4 h3) (connected h4 h5) (connected h4 h6) (connected h4 h7) (connected h4 h8) (connected h4 h9) (connected h4 h10) (connected h4 h11) (connected h4 h12) (connected h4 h13) (connected h4 h14) (connected h4 h15) (connected h4 h16) (connected h4 h17) (connected h4 h18) (connected h4 h19) (connected h4 h20)
    (connected h5 h1) (connected h5 h2) (connected h5 h3) (connected h5 h4) (connected h5 h6) (connected h5 h7) (connected h5 h8) (connected h5 h9) (connected h5 h10) (connected h5 h11) (connected h5 h12) (connected h5 h13) (connected h5 h14) (connected h5 h15) (connected h5 h16) (connected h5 h17) (connected h5 h18) (connected h5 h19) (connected h5 h20)
    ;; (Optionnel : tu peux ajouter h6..h20 pour saturer davantage)

    ;; --- CONNEXIONS SATELLITES (60 satellites connectés à 20 hubs bi-directionnel) ---
    ;; Chaque ligne ci-dessous crée 1600 actions de move (80 robots * 20 Hubs)
    (connected s1 h1) (connected s1 h2) (connected h1 s1) (connected h2 s1)
    (connected s2 h2) (connected s2 h3) (connected h2 s2) (connected h3 s2)
    (connected s3 h3) (connected s3 h4) (connected h3 s3) (connected h4 s3)
    (connected s4 h4) (connected s4 h5) (connected h4 s4) (connected h5 s4)
    (connected s5 h5) (connected s5 h6) (connected h5 s5) (connected h6 s5)
    (connected s6 h6) (connected s6 h7) (connected h6 s6) (connected h7 s6)
    (connected s7 h7) (connected s7 h8) (connected h7 s7) (connected h8 s7)
    (connected s8 h8) (connected s8 h9) (connected h8 s8) (connected h9 s8)
    (connected s9 h9) (connected s9 h10) (connected h9 s9) (connected h10 s9)
    (connected s10 h10) (connected s10 h1) (connected h10 s10) (connected h1 s10)

    ;; Répétition pour s11-s60 (Tu peux copier-coller les lignes du dessus en changeant juste le sX)
    (connected s11 h1) (connected s12 h2) (connected s13 h3) (connected s14 h4) (connected s15 h5)
    (connected s16 h6) (connected s17 h7) (connected s18 h8) (connected s19 h9) (connected s20 h10)
    (connected s21 h11) (connected s22 h12) (connected s23 h13) (connected s24 h14) (connected s25 h15)
    (connected s26 h16) (connected s27 h17) (connected s28 h18) (connected s29 h19) (connected s30 h20)
  )

  (:goal (and (task-completed s30)))
)
