use crate::aiplan4rust::grounding::problem::registry::value::error::ValueRegistryError;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lang::{ObjectId, Type, TypeId, TypedSymbol};

/// **Test 1: Linear Inheritance Chain**
///
/// * **Objective**: Verify that an object declared at the leaf of a multi-level
///   inheritance chain is correctly propagated up to the root, ensuring
///   visibility in every intermediate type's domain.
///
/// * **Input**:
///     - Hierarchy: `Object (root) <- Physical <- Vehicle <- Truck`
///     - Constant: `truck_1` of type `Truck`
///
/// * **Expected Output**:
///     - Successful registry build.
///     - `truck_1` must appear in the domains of `Truck`, `Vehicle`, `Physical`, and `Object`.
///     - `unique_objects_count` must be 1.
///     - `storage_size` must be 4 (due to flattened redundancy for $O(1)$ access).
#[test]
fn test_build_linear_inheritance_chain() {
    // Type IDs
    let t_obj = TypeId::from(0);
    let t_phy = TypeId::from(1);
    let t_veh = TypeId::from(2);
    let t_tru = TypeId::from(3);

    // Definitions: Object <- Physical <- Vehicle <- Truck
    let type_defs = vec![
        TypedSymbol::new(t_obj, Type::root()),
        TypedSymbol::new(t_phy, Type::primitive(t_obj)),
        TypedSymbol::new(t_veh, Type::primitive(t_phy)),
        TypedSymbol::new(t_tru, Type::primitive(t_veh)),
    ];

    // Single object defined at the leaf
    let o_truck = ObjectId::from(100);
    let object_defs = vec![TypedSymbol::new(o_truck, Type::primitive(t_tru))];

    let registry = ValueRegistry::build(&type_defs, &object_defs)
        .expect("Registry build should succeed for a valid linear hierarchy");

    // 1. Verify upward propagation: the object must exist in all ancestor domains
    assert_eq!(
        registry.get_primitive_type_domain(t_tru).unwrap(),
        &[o_truck]
    );
    assert_eq!(
        registry.get_primitive_type_domain(t_veh).unwrap(),
        &[o_truck]
    );
    assert_eq!(
        registry.get_primitive_type_domain(t_phy).unwrap(),
        &[o_truck]
    );
    assert_eq!(
        registry.get_primitive_type_domain(t_obj).unwrap(),
        &[o_truck]
    );

    // 2. Verify internal memory structure
    assert_eq!(
        registry.unique_objects_count(),
        1,
        "There is only 1 unique constant"
    );
    assert_eq!(
        registry.storage_size(),
        4,
        "The object is duplicated 4 times to allow O(1) access for each type"
    );
}

/// **Test 2: Disjoint Branches and Root Merging**
///
/// * **Objective**: Ensure that objects belonging to parallel branches (sibling types)
///   remain isolated from one another, while the common ancestor correctly
///   aggregates objects from all its descendants.
///
/// * **Input**:
///     - Hierarchy: `Object (root)` has two children: `Physical` and `Location`.
///     - Objects: `obj_0` of type `Physical`, `obj_20` of type `Location`.
///
/// * **Expected Output**:
///     - `Physical` domain contains only `obj_0`.
///     - `Location` domain contains only `obj_20`.
///     - `Object` (root) domain contains both `obj_0` and `obj_20`.
///     - `unique_objects_count` must be 2.
#[test]
fn test_build_disjoint_branches() {
    let t_obj = TypeId::from(0);
    let t_phy = TypeId::from(1);
    let t_loc = TypeId::from(2);

    // Definitions: Object is root, Physical and Location are independent children
    let type_defs = vec![
        TypedSymbol::new(t_obj, Type::root()),
        TypedSymbol::new(t_phy, Type::primitive(t_obj)),
        TypedSymbol::new(t_loc, Type::primitive(t_obj)),
    ];

    let o_phy = ObjectId::from(0);
    let o_loc = ObjectId::from(20);

    let object_defs = vec![
        TypedSymbol::new(o_phy, Type::primitive(t_phy)),
        TypedSymbol::new(o_loc, Type::primitive(t_loc)),
    ];

    let registry = ValueRegistry::build(&type_defs, &object_defs)
        .expect("Registry build should succeed for valid disjoint branches");

    // 1. Isolation Check: Physical branch should not see Location objects
    let phy_domain = registry
        .get_primitive_type_domain(t_phy)
        .expect("Physical domain missing");
    assert!(phy_domain.contains(&o_phy));
    assert!(
        !phy_domain.contains(&o_loc),
        "Physical domain should not include Location objects"
    );

    // 2. Isolation Check: Location branch should not see Physical objects
    let loc_domain = registry
        .get_primitive_type_domain(t_loc)
        .expect("Location domain missing");
    assert!(loc_domain.contains(&o_loc));
    assert!(
        !loc_domain.contains(&o_phy),
        "Location domain should not include Physical objects"
    );

    // 3. Merge Check: Root should aggregate objects from all branches
    let obj_domain = registry
        .get_primitive_type_domain(t_obj)
        .expect("Root domain missing");
    assert_eq!(
        obj_domain.len(),
        2,
        "Root domain must contain the union of all child objects"
    );
    assert!(obj_domain.contains(&o_phy));
    assert!(obj_domain.contains(&o_loc));

    // Global consistency
    assert_eq!(registry.unique_objects_count(), 2);
}

/// **Test 3: Empty Abstract Type**
///
/// * **Objective**: Ensure that the registry can be built correctly even when
///   some types (or all types) have no associated objects. This is common for
///   abstract types in PDDL.
///
/// * **Input**:
///     - Hierarchy: `Object (root) <- AbstractType`
///     - Objects: Empty list.
///
/// * **Expected Output**:
///     - Successful registry build.
///     - `AbstractType` domain exists but is empty.
///     - `get_range` returns a valid range of length 0.
///     - `storage_size` and `unique_objects_count` are both 0.
#[test]
fn test_build_empty_abstract_type() {
    let t_obj = TypeId::from(0);
    let t_abstract = TypeId::from(1);

    // Definitions: Simple hierarchy with no objects assigned
    let type_defs = vec![
        TypedSymbol::new(t_obj, Type::root()),
        TypedSymbol::new(t_abstract, Type::primitive(t_obj)),
    ];

    let registry = ValueRegistry::build(&type_defs, &vec![])
        .expect("Registry build should succeed even with no objects");

    // 1. Domain Check: The type exists in the registry but its domain is empty
    let domain = registry
        .get_primitive_type_domain(t_abstract)
        .expect("Domain for AbstractType should be accessible");
    assert!(
        domain.is_empty(),
        "Domain should be empty for a type with no instances"
    );

    // 2. Range Check: Range should be valid but signify zero elements
    let range = registry
        .get_range(t_abstract)
        .expect("Range should be defined even for empty types");
    assert_eq!(range.len(), 0, "Range length must be 0 for empty domains");

    // 3. Global Consistency
    assert_eq!(registry.storage_size(), 0, "No objects should be stored");
    assert_eq!(
        registry.unique_objects_count(),
        0,
        "Unique object count should be 0"
    );
}

/// **Test 4: Forest Structure (Multiple Independent Roots)**
///
/// * **Objective**: Verify that the registry correctly handles a "forest" topology
///   where multiple types are declared as roots. Each tree in the forest must
///   manage its own objects independently in memory.
///
/// * **Input**:
///     - Hierarchy: Two independent roots, `RootA` and `RootB`.
///     - Objects: `obj_1` belonging to `RootA`, `obj_2` belonging to `RootB`.
///
/// * **Expected Output**:
///     - Successful registry build.
///     - `RootA` domain contains exactly `obj_1`.
///     - `RootB` domain contains exactly `obj_2`.
///     - Memory ranges for `RootA` and `RootB` must be disjoint (no overlapping storage).
///     - `unique_objects_count` and `storage_size` are both 2.
#[test]
fn test_build_multiple_roots_forest() {
    let t_root_a = TypeId::from(0);
    let t_root_b = TypeId::from(1);

    // Definitions: Two independent roots with no common ancestor
    let type_defs = vec![
        TypedSymbol::new(t_root_a, Type::root()),
        TypedSymbol::new(t_root_b, Type::root()),
    ];

    let o_1 = ObjectId::from(1);
    let o_2 = ObjectId::from(2);

    let object_defs = vec![
        TypedSymbol::new(o_1, Type::primitive(t_root_a)),
        TypedSymbol::new(o_2, Type::primitive(t_root_b)),
    ];

    let registry = ValueRegistry::build(&type_defs, &object_defs)
        .expect("Registry build should succeed for a forest structure");

    // 1. Domain Check: Verify each root contains its respective objects
    assert_eq!(
        registry
            .get_primitive_type_domain(t_root_a)
            .expect("Domain A missing"),
        &[o_1],
        "RootA should only contain obj_1"
    );
    assert_eq!(
        registry
            .get_primitive_type_domain(t_root_b)
            .expect("Domain B missing"),
        &[o_2],
        "RootB should only contain obj_2"
    );

    // 2. Memory Safety Check: Ensure ranges in the global buffer are disjoint
    let r_a = registry.get_range(t_root_a).expect("Range A missing");
    let r_b = registry.get_range(t_root_b).expect("Range B missing");

    assert!(
        r_a.end() <= r_b.start() || r_b.end() <= r_a.start(),
        "Memory ranges for independent roots must not overlap in the flat buffer"
    );

    // 3. Global Consistency
    assert_eq!(
        registry.unique_objects_count(),
        2,
        "Total unique objects should be 2"
    );
    assert_eq!(
        registry.storage_size(),
        2,
        "Total storage size should be 2 (no duplication needed here)"
    );
}
/// **Test 5: Diamond Inheritance (Deduplication)**
///
/// * **Objective**: Verify that the registry correctly handles multiple inheritance paths
///   to the same ancestor. Even if an object's type has two parents that share a common
///   ancestor (a diamond shape), the object must only appear once in that ancestor's domain.
///
/// * **Input**:
///     - Hierarchy: `A (root) <- (B, C) <- D`. Type `D` inherits from both `B` and `C`.
///     - Objects: `obj_400` belonging to leaf type `D`.
///
/// * **Expected Output**:
///     - Successful registry build.
///     - `obj_400` is present in domains `D`, `B`, `C`, and `A`.
///     - The domain of `A` contains only **one** instance of `obj_400` (strict deduplication).
///     - `storage_size` is exactly 4 (A, B, C, and D each have their own slice).
#[test]
fn test_build_diamond_inheritance_deduplication() {
    let t_a = TypeId::from(0);
    let t_b = TypeId::from(1);
    let t_c = TypeId::from(2);
    let t_d = TypeId::from(3);

    // Definitions: D inherits from B and C, which both inherit from A.
    let type_defs = vec![
        TypedSymbol::new(t_a, Type::root()),
        TypedSymbol::new(t_b, Type::primitive(t_a)),
        TypedSymbol::new(t_c, Type::primitive(t_a)),
        TypedSymbol::new(t_d, Type::either(&[t_b, t_c])),
    ];

    let o_d = ObjectId::from(400);
    let object_defs = vec![TypedSymbol::new(o_d, Type::primitive(t_d))];

    let registry = ValueRegistry::build(&type_defs, &object_defs)
        .expect("Registry build should succeed for a diamond hierarchy");

    // 1. Logic Check: Object must be present in the entire lineage
    assert_eq!(
        registry
            .get_primitive_type_domain(t_d)
            .expect("Domain D missing"),
        &[o_d]
    );
    assert_eq!(
        registry
            .get_primitive_type_domain(t_b)
            .expect("Domain B missing"),
        &[o_d]
    );
    assert_eq!(
        registry
            .get_primitive_type_domain(t_c)
            .expect("Domain C missing"),
        &[o_d]
    );
    assert_eq!(
        registry
            .get_primitive_type_domain(t_a)
            .expect("Domain A missing"),
        &[o_d]
    );

    // 2. Deduplication Check: Common ancestor A must not have duplicates
    let domain_a = registry
        .get_primitive_type_domain(t_a)
        .expect("Domain A missing");
    assert_eq!(
        domain_a.len(),
        1,
        "Common ancestor should deduplicate objects reached via multiple paths"
    );

    // 3. Consistency Checks
    assert_eq!(
        registry.unique_objects_count(),
        1,
        "Only one unique object exists"
    );
    assert_eq!(
        registry.storage_size(),
        4,
        "Storage size must be 4 (1 per type) to maintain O(1) domain access"
    );
}
/// **Test 6: Indirect Cycle Detection**
///
/// * **Objective**: Verify that the builder detects circular dependencies in the
///   type hierarchy. Cycles must be forbidden as they would lead to infinite
///   recursion when propagating objects up the inheritance chain.
///
/// * **Input**:
///     - Hierarchy: `Type A` inherits from `Type B`, and `Type B` inherits from `Type A`.
///     - Objects: None.
///
/// * **Expected Output**:
///     - The build must fail.
///     - The returned error must be `ValueRegistryError::CycleDetected`.
#[test]
fn test_build_error_on_indirect_cycle() {
    let t_a = TypeId::from(0);
    let t_b = TypeId::from(1);

    // Definitions: Mutual inheritance (A -> B and B -> A)
    let type_defs = vec![
        TypedSymbol::new(t_a, Type::primitive(t_b)),
        TypedSymbol::new(t_b, Type::primitive(t_a)),
    ];

    let result = ValueRegistry::build(&type_defs, &vec![]);

    // 1. Failure Check: Build must return an error
    assert!(
        result.is_err(),
        "Registry build should fail when a circular dependency is detected"
    );

    // 2. Error Type Check: Ensure the error specifically identifies a cycle
    if let Err(e) = result {
        assert!(
            matches!(e, ValueRegistryError::CycleDetected(_)),
            "Expected CycleDetected error, but received: {:?}",
            e
        );
    }
}

/// **Test 7: Direct Cycle Detection (Self-Inheritance)**
///
/// * **Objective**: Verify that the builder correctly identifies and rejects a type
///   that inherits directly from itself. This is the simplest form of a cycle.
///
/// * **Input**:
///     - Hierarchy: `Type A` inherits from `Type A`.
///     - Objects: None.
///
/// * **Expected Output**:
///     - The build must fail.
///     - The error must be `ValueRegistryError::CycleDetected`.
///     - The error should specifically identify `Type A` as the source of the cycle.
#[test]
fn test_build_error_on_direct_cycle() {
    // Configuration: A -> A
    let t_a = TypeId::from(0);

    let type_defs = vec![TypedSymbol::new(t_a, Type::primitive(t_a))];

    let result = ValueRegistry::build(&type_defs, &vec![]);

    // 1. Failure Check: Self-inheritance is a logical error in PDDL
    assert!(
        result.is_err(),
        "Registry build should fail when a type inherits from itself"
    );

    // 2. Error Content Check: Ensure the offending TypeId is reported
    if let Err(e) = result {
        assert!(
            matches!(e, ValueRegistryError::CycleDetected(id) if id == t_a),
            "Expected CycleDetected(TypeId(0)), but received: {:?}",
            e
        );
    }
}

/// **Test 8: Either Type Normalization (Parser-Generated Union)**
///
/// * **Objective**: Verify that the registry correctly handles "union types" created
///   by the parser to represent `(either T1 T2)`. This confirms that the registry
///   logic supports multiple inheritance and propagates objects from a union type
///   up to all its constituent types.
///
/// * **Input**:
///     - Hierarchy: `Vehicle (root)`, `Truck (child of Vehicle)`.
///     - Union Type: `Union_Type` (inherits from both `Vehicle` and `Truck`).
///     - Objects: `obj_10` assigned specifically to `Union_Type`.
///
/// * **Expected Output**:
///     - Successful registry build.
///     - `obj_10` must propagate up to both `Truck` and `Vehicle`.
///     - `obj_10` must be accessible via `Union_Type` directly.
#[test]
fn test_build_either_normalized_via_parser_logic() {
    let t_veh = TypeId::from(0);
    let t_tru = TypeId::from(1);
    let t_union = TypeId::from(2); // The artificial type created by the parser

    // Definitions: Union inherits from both Vehicle and Truck
    let type_defs = vec![
        TypedSymbol::new(t_veh, Type::root()),
        TypedSymbol::new(t_tru, Type::primitive(t_veh)),
        TypedSymbol::new(t_union, Type::either(&[t_veh, t_tru])),
    ];

    let o_1 = ObjectId::from(10);
    let object_defs = vec![
        // Object is assigned to the union type
        TypedSymbol::new(o_1, Type::primitive(t_union)),
    ];

    let registry = ValueRegistry::build(&type_defs, &object_defs)
        .expect("Registry build should succeed when objects are assigned to union types");

    // 1. Domain Check: Verify upward propagation through the union
    assert_eq!(
        registry
            .get_primitive_type_domain(t_union)
            .expect("Union domain missing"),
        &[o_1]
    );
    assert_eq!(
        registry
            .get_primitive_type_domain(t_tru)
            .expect("Truck domain missing"),
        &[o_1]
    );
    assert_eq!(
        registry
            .get_primitive_type_domain(t_veh)
            .expect("Vehicle domain missing"),
        &[o_1]
    );

    // 2. Consistency Check
    assert_eq!(registry.unique_objects_count(), 1);
}

/// **Test 9: Out of Bounds Type Reference (Runtime Safety)**
///
/// * **Objective**: Verify that the registry safely handles queries for `TypeId`s
///   that do not exist in the system. The registry must return a structured
///   error instead of panicking with an "index out of bounds" exception.
///
/// * **Input**:
///     - A registry built with only one type (`TypeId(0)`).
///     - A runtime query for `TypeId(999)`.
///
/// * **Expected Output**:
///     - `get_primitive_type_domain` must return `Err(ValueRegistryError::TypeIdOutOfBounds)`.
///     - The error must correctly identify the faulty ID (`999`).
#[test]
fn test_query_error_on_out_of_bounds_type() {
    // 1. Setup: Build a registry with a single known type
    let type_defs = vec![TypedSymbol::new(TypeId::from(0), Type::root())];
    let registry = ValueRegistry::build(&type_defs, &vec![])
        .expect("Registry build should succeed for a single valid type");

    // 2. Query: Request a domain for a non-existent TypeId
    let invalid_id = TypeId::from(999);
    let result = registry.get_primitive_type_domain(invalid_id);

    // 3. Validation: Check for graceful error handling instead of panic
    assert!(
        result.is_err(),
        "Querying an unknown TypeId should return an Err, not panic"
    );

    if let Err(e) = result {
        assert!(
            matches!(e, ValueRegistryError::TypeIdOutOfBounds(id, _) if id == invalid_id),
            "Expected TypeIdOutOfBounds(999), but received: {:?}",
            e
        );
    }
}

/// **Test 10: Object Order Preservation**
///
/// * **Objective**: Ensure that the registry preserves the declaration order of
///   objects within a type domain. Deterministic ordering is crucial for
///   consistent grounding and predictable behavior across multiple runs.
///
/// * **Input**:
///     - A single type `Truck`.
///     - Three objects `o1`, `o2`, `o3` defined in that specific sequence.
///
/// * **Expected Output**:
///     - Successful registry build.
///     - The domain for `Truck` must return objects in the exact order: `[o1, o2, o3]`.
#[test]
fn test_build_preserves_object_declaration_order() {
    let t_tru = TypeId::from(0);
    let type_defs = vec![TypedSymbol::new(t_tru, Type::root())];

    let o1 = ObjectId::from(1);
    let o2 = ObjectId::from(2);
    let o3 = ObjectId::from(3);

    let object_defs = vec![
        TypedSymbol::new(o1, Type::primitive(t_tru)),
        TypedSymbol::new(o2, Type::primitive(t_tru)),
        TypedSymbol::new(o3, Type::primitive(t_tru)),
    ];

    let registry = ValueRegistry::build(&type_defs, &object_defs)
        .expect("Registry build should succeed and preserve order");

    // Validation: Check that the slice matches the input sequence exactly
    assert_eq!(
        registry
            .get_primitive_type_domain(t_tru)
            .expect("Domain missing"),
        &[o1, o2, o3],
        "The registry must maintain the order in which objects were defined"
    );
}

/// **Test 11: Empty Root Validation**
///
/// * **Objective**: Verify that the registry can be successfully initialized even
///   if the root type has no associated objects. A hierarchy with types but no
///   instances is a valid state for the grounding engine.
///
/// * **Input**:
///     - A single root type `Root`.
///     - No object definitions.
///
/// * **Expected Output**:
///     - Successful registry build.
///     - The domain for `Root` must be an empty slice.
#[test]
fn test_build_succeeds_with_empty_root() {
    let t_root = TypeId::from(0);

    // Definition: A single root type with no children and no objects
    let type_defs = vec![TypedSymbol::new(t_root, Type::root())];

    let registry = ValueRegistry::build(&type_defs, &vec![])
        .expect("Registry build should succeed even if the root type is empty");

    // Validation: Ensure the domain is accessible and empty
    let domain = registry
        .get_primitive_type_domain(t_root)
        .expect("Domain for the root type should exist");

    assert!(
        domain.is_empty(),
        "The domain must be empty when no objects are provided"
    );
}

/// **Test 12: Invalid Parent Type Reference (Build Safety)**
///
/// * **Objective**: Ensure the builder validates the integrity of the inheritance
///   graph. If a type defines a parent that does not exist in the provided
///   type definitions, the build must fail and identify the missing ID.
///
/// * **Input**:
///     - `Type(0)` which attempts to inherit from `Type(99)`.
///     - `Type(99)` is not present in the `type_defs` list.
///
/// * **Expected Output**:
///     - The build must return `Err(ValueRegistryError::TypeIdOutOfBounds)`.
///     - The error must specifically capture the ID `99`.
#[test]
fn test_build_error_on_invalid_parent_reference() {
    let t_exists = TypeId::from(0);
    let t_missing = TypeId::from(99); // This ID is never defined as a symbol

    // Definition: A valid symbol pointing to an undefined parent ID
    let type_defs = vec![TypedSymbol::new(t_exists, Type::primitive(t_missing))];

    let result = ValueRegistry::build(&type_defs, &vec![]);

    // 1. Failure Check
    assert!(
        result.is_err(),
        "Registry build should fail if a parent type is missing from definitions"
    );

    // 2. Specificity Check: We verify that the error points to the missing parent ID
    if let Err(e) = result {
        assert!(
            matches!(e, ValueRegistryError::TypeIdOutOfBounds(id, _) if id == t_missing),
            "Expected TypeIdOutOfBounds(99) for the missing parent, but received: {:?}",
            e
        );
    }
}

/// **Test 13: Object with Invalid Type Reference**
///
/// * **Objective**: Verify that the builder identifies and rejects objects assigned
///   to a `TypeId` that was never defined. This ensures data integrity between
///   the problem constants and the domain hierarchy.
///
/// * **Input**:
///     - A single valid type `TypeId(0)`.
///     - An object definition referencing `TypeId(99)`.
///
/// * **Expected Output**:
///     - The build must fail.
///     - The error must be `ValueRegistryError::TypeIdOutOfBounds`.
///     - The error must specifically point to the missing `TypeId(99)`.
#[test]
fn test_build_error_on_object_with_invalid_type() {
    let t_exists = TypeId::from(0);
    let t_missing = TypeId::from(99);

    // Definitions: Only Type 0 exists
    let type_defs = vec![TypedSymbol::new(t_exists, Type::root())];

    // Object 1 tries to belong to non-existent Type 99
    let object_defs = vec![TypedSymbol::new(
        ObjectId::from(1),
        Type::primitive(t_missing),
    )];

    let result = ValueRegistry::build(&type_defs, &object_defs);

    // 1. Failure Check: Object-to-Type mapping must be valid
    assert!(
        result.is_err(),
        "Registry build should fail if an object references an undefined TypeId"
    );

    // 2. Error Detail Check: Ensure the offending ID is captured
    if let Err(e) = result {
        assert!(
            matches!(e, ValueRegistryError::TypeIdOutOfBounds(id, _) if id == t_missing),
            "Expected TypeIdOutOfBounds(99), but received: {:?}",
            e
        );
    }
}
