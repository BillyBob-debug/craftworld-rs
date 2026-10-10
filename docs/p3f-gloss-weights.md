# P3f: gloss weights carry the stored colour .w

Moon oracle (#110): the live lighting vec4s equal the stored vec4s times a factor, rgb and
.w alike (sun 0.3 x mult x exposure, ambient 0.45 x exposure, rims 0.6 x exposure; fog
colour unscaled). The rgb already followed that. The .w components weight the gloss term
(`light.w` in the brdf reconstruction) and were computed as factor only, i.e. with a stored
.w of 1. They now use stored .w x factor per colour, with rim and rim2 weighted separately
(the Moon rim .w is 1.5, rim2 1.0). Stored .w that is not finite or not positive counts as 1.
The stored .w does not scale the rgb (verified against the Moon block, rim rgb = stored x 0.63).

Evidence: Moon plain-background preset 1 against the live block, PC comment on #110. The
Pod's sun .w (0.3725 = 0.3 x 1.0206 x 1.2165) is consistent with a stored sun .w of 1.
