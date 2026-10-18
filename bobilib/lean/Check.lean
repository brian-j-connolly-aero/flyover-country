import Bobi

/-! Lists the axioms each main theorem depends on (run `lake env lean Check.lean`).
Every line should show only `propext`, `Classical.choice` and `Quot.sound`. -/

#print axioms Bobi.follower_antitone
#print axioms Bobi.bestAvoiding_antitone
#print axioms Bobi.bestAvoiding_eq_sup_sdiff
#print axioms Bobi.packings_downward_closed
#print axioms Bobi.K_eq_bestAvoiding
#print axioms Bobi.swap_block
#print axioms Bobi.cheapest_first
#print axioms Bobi.window_extend
#print axioms Bobi.window_extend_all
#print axioms Bobi.window_survivors
#print axioms Bobi.online_le_K
#print axioms Bobi.online_le_K_any
#print axioms Bobi.online_lower_bound
#print axioms Bobi.table1_online
#print axioms Bobi.table1_block
#print axioms Bobi.table1_opt
#print axioms Bobi.H_sound
#print axioms Bobi.H_sound_any
#print axioms Bobi.covers_sound
