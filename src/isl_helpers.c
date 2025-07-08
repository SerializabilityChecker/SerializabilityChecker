#include <isl/ctx.h>
#include <isl/space.h>
#include <isl/set.h>
#include <isl/aff.h>
#include <isl/list.h>

// Define a struct to return the two resulting sets
typedef struct {
    isl_set *set1;
    isl_set *set2;
    int error; // 0 on success, non-zero on error
} HarmonizeResult;

// Helper to create the preimage multi_aff assuming positional correspondence
// Consumes original_space, returns new multi_aff or NULL on error
static isl_multi_aff *create_preimage_map_positional(
    isl_space *target_space,    // Borrows
    isl_space *original_space) // Consumes
{
    if (!target_space || !original_space) {
        isl_space_free(original_space);
        return NULL;
    }

    isl_ctx *ctx = isl_space_get_ctx(target_space);
    isl_space *map_space = isl_space_map_from_domain_and_range(
        isl_space_copy(target_space),
        isl_space_copy(original_space)); // range is original space

    isl_size n_original_dims = isl_space_dim(original_space, isl_dim_set);
    if (n_original_dims < 0) {
        isl_space_free(original_space);
        isl_space_free(map_space);
        return NULL;
    }

    isl_aff_list *aff_list = isl_aff_list_alloc(ctx, n_original_dims);
    if (!aff_list) {
         isl_space_free(original_space);
         isl_space_free(map_space);
         return NULL;
    }

    for (int i = 0; i < n_original_dims; ++i) {
        // Assume target_space has dimensions corresponding to original_mapping
        // at the *same first* n_original_dims positions
        isl_local_space *ls = isl_local_space_from_space(isl_space_copy(target_space));
        isl_aff *aff = isl_aff_var_on_domain(ls, isl_dim_set, i); // Get target_dim[i]
        aff_list = isl_aff_list_add(aff_list, aff); // Add it to output list position 'i'
    }

    isl_multi_aff *ma = isl_multi_aff_from_aff_list(map_space, aff_list); // Consumes map_space and aff_list

    isl_space_free(original_space); // Free the input copy
    return ma;
}


// The C wrapper function called from Rust
// Assumes target_space dimensions are ordered according to the combined Rust mapping
// Consumes set1_in and set2_in, returns owned pointers in the struct
HarmonizeResult rust_harmonize_sets(
    isl_set *set1_in,
    isl_set *set2_in,
    isl_space *target_space) // Borrows target_space
{
    HarmonizeResult result = {NULL, NULL, 1}; // Default to error

    if (!set1_in || !set2_in || !target_space) {
        isl_set_free(set1_in);
        isl_set_free(set2_in);
        return result; // Return error state
    }

    isl_space *space1 = isl_set_get_space(set1_in);
    isl_space *space2 = isl_set_get_space(set2_in);

    // --- Align parameters first (crucial!) ---
    // This ensures parameter spaces match before creating preimage maps
    isl_set *set1_p = isl_set_align_params(set1_in, isl_space_copy(target_space));
    isl_set *set2_p = isl_set_align_params(set2_in, isl_space_copy(target_space));
    // set1_in and set2_in are consumed by align_params

    if (!set1_p || !set2_p) {
        isl_space_free(space1);
        isl_space_free(space2);
        isl_set_free(set1_p); // Free if one succeeded but other failed
        isl_set_free(set2_p);
        return result;
    }
    // Use the spaces AFTER parameter alignment for map creation
    isl_space *space1_p = isl_set_get_space(set1_p);
    isl_space *space2_p = isl_set_get_space(set2_p);
    // --- End Parameter Alignment ---

    isl_multi_aff *ma1 = create_preimage_map_positional(target_space, space1_p); // Consumes space1_p
    isl_multi_aff *ma2 = create_preimage_map_positional(target_space, space2_p); // Consumes space2_p

    if (!ma1 || !ma2) {
        isl_multi_aff_free(ma1); // Free whichever succeeded
        isl_multi_aff_free(ma2);
        isl_set_free(set1_p); // Free the aligned sets
        isl_set_free(set2_p);
        isl_space_free(space1); // Free original space copies
        isl_space_free(space2);
        return result;
    }

    // Compute preimages (consumes set1_p, set2_p, ma1, ma2)
    result.set1 = isl_set_preimage_multi_aff(set1_p, ma1);
    result.set2 = isl_set_preimage_multi_aff(set2_p, ma2);

    // Check for errors during preimage
    if (!result.set1 || !result.set2) {
        isl_set_free(result.set1); // Free whichever succeeded
        isl_set_free(result.set2);
        result.set1 = NULL;
        result.set2 = NULL;
        // Error already set to 1
    } else {
        result.error = 0; // Success!
    }

    // Cleanup original space copies
    isl_space_free(space1);
    isl_space_free(space2);

    return result;
}

// Helper to create preimage map with explicit dimension mapping
static isl_multi_aff *create_preimage_map_with_mapping(
    isl_space *target_space,    // Borrows
    isl_space *original_space,  // Consumes
    const int *mapping_indices, // Array mapping original dims to target positions
    int original_dims)          // Number of dimensions in original space
{
    if (!target_space || !original_space || !mapping_indices) {
        isl_space_free(original_space);
        return NULL;
    }

    isl_ctx *ctx = isl_space_get_ctx(target_space);
    isl_space *map_space = isl_space_map_from_domain_and_range(
        isl_space_copy(target_space),
        isl_space_copy(original_space));

    isl_aff_list *aff_list = isl_aff_list_alloc(ctx, original_dims);
    if (!aff_list) {
        isl_space_free(original_space);
        isl_space_free(map_space);
        return NULL;
    }

    for (int i = 0; i < original_dims; ++i) {
        // Map original dimension i to target dimension mapping_indices[i]
        int target_dim = mapping_indices[i];
        isl_local_space *ls = isl_local_space_from_space(isl_space_copy(target_space));
        isl_aff *aff = isl_aff_var_on_domain(ls, isl_dim_set, target_dim);
        aff_list = isl_aff_list_add(aff_list, aff);
    }

    isl_multi_aff *ma = isl_multi_aff_from_aff_list(map_space, aff_list);
    isl_space_free(original_space);
    return ma;
}

// Improved harmonization function with explicit mapping
HarmonizeResult rust_harmonize_sets_with_mapping(
    isl_set *set1_in,
    isl_set *set2_in,
    isl_space *target_space,
    const int *set1_indices,
    int set1_dims,
    const int *set2_indices,
    int set2_dims)
{
    HarmonizeResult result = {NULL, NULL, 1}; // Default to error

    if (!set1_in || !set2_in || !target_space || !set1_indices || !set2_indices) {
        isl_set_free(set1_in);
        isl_set_free(set2_in);
        return result;
    }

    isl_space *space1 = isl_set_get_space(set1_in);
    isl_space *space2 = isl_set_get_space(set2_in);

    // Align parameters first
    isl_set *set1_p = isl_set_align_params(set1_in, isl_space_copy(target_space));
    isl_set *set2_p = isl_set_align_params(set2_in, isl_space_copy(target_space));

    if (!set1_p || !set2_p) {
        isl_space_free(space1);
        isl_space_free(space2);
        isl_set_free(set1_p);
        isl_set_free(set2_p);
        return result;
    }

    isl_space *space1_p = isl_set_get_space(set1_p);
    isl_space *space2_p = isl_set_get_space(set2_p);

    // Create preimage maps with explicit dimension mapping
    isl_multi_aff *ma1 = create_preimage_map_with_mapping(target_space, space1_p, set1_indices, set1_dims);
    isl_multi_aff *ma2 = create_preimage_map_with_mapping(target_space, space2_p, set2_indices, set2_dims);

    if (!ma1 || !ma2) {
        isl_multi_aff_free(ma1);
        isl_multi_aff_free(ma2);
        isl_set_free(set1_p);
        isl_set_free(set2_p);
        isl_space_free(space1);
        isl_space_free(space2);
        return result;
    }

    // Compute preimages
    result.set1 = isl_set_preimage_multi_aff(set1_p, ma1);
    result.set2 = isl_set_preimage_multi_aff(set2_p, ma2);

    if (!result.set1 || !result.set2) {
        isl_set_free(result.set1);
        isl_set_free(result.set2);
        result.set1 = NULL;
        result.set2 = NULL;
        // Error already set to 1
    } else {
        result.error = 0; // Success!
    }

    // Cleanup
    isl_space_free(space1);
    isl_space_free(space2);

    return result;
}