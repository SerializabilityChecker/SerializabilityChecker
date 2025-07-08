// Wrapper header for generating isl bindings

#include <isl/ctx.h>
#include <isl/options.h>
#include <isl/val.h>
#include <isl/space.h>
#include <isl/set.h>
#include <isl/constraint.h>
#include <isl/space_type.h>
#include <isl/aff.h>
#include <isl/list.h>

typedef struct {
    isl_set *set1;
    isl_set *set2;
    int error;
} HarmonizeResult;

HarmonizeResult rust_harmonize_sets(
    isl_set *set1_in,
    isl_set *set2_in,
    isl_space *target_space);

// New improved harmonization function that takes mapping information
HarmonizeResult rust_harmonize_sets_with_mapping(
    isl_set *set1_in,
    isl_set *set2_in,
    isl_space *target_space,
    const int *set1_indices,  // Array mapping set1 dimensions to target positions
    int set1_dims,            // Number of dimensions in set1
    const int *set2_indices,  // Array mapping set2 dimensions to target positions  
    int set2_dims             // Number of dimensions in set2
);