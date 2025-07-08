#!/usr/bin/env python3
import subprocess
import os
import json
import sys
from pathlib import Path
import re

def run_serializability_checker(json_file):
    """Run the serializability checker on a JSON file and capture output"""
    try:
        result = subprocess.run(
            ["cargo", "run", "--", json_file],
            capture_output=True,
            text=True,
            timeout=30
        )
        return result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired:
        return "", "TIMEOUT", -1
    except Exception as e:
        return "", f"ERROR: {str(e)}", -1

def check_proof_validity(stdout):
    """Check if the proof verification indicates the proof is valid"""
    # Look for proof certificate verification result
    if "✅ Proof certificate is VALID" in stdout:
        return True, "valid"
    elif "❌ Proof certificate is INVALID" in stdout:
        return False, "invalid"
    elif "Proof Certificate Verification:" in stdout:
        # Proof verification section exists but no clear result
        return False, "unclear"
    else:
        # No proof verification section
        return None, "no_verification"

def analyze_result(stdout, stderr, returncode):
    """Analyze the output to determine if the result and proof are correct"""
    info = {}
    
    # Check for errors
    if returncode != 0 or "ERROR" in stderr or "TIMEOUT" in stderr:
        info["status"] = "error"
        info["error"] = stderr
        return info
    
    # Determine if serializable or not
    if "✅ The network system IS serializable" in stdout:
        info["result"] = "serializable"
    elif "❌ The network system is NOT serializable" in stdout:
        info["result"] = "not_serializable"
    else:
        info["result"] = "unknown"
    
    # Check proof validity for serializable cases
    if info["result"] == "serializable":
        proof_valid, proof_status = check_proof_validity(stdout)
        info["proof_valid"] = proof_valid
        info["proof_status"] = proof_status
        
        # Check if we have NS-level invariants
        if "NS-Level Invariants:" in stdout:
            info["has_ns_invariants"] = True
        else:
            info["has_ns_invariants"] = False
            
        # Check if initial state check passed
        if "✓ Initial state satisfies the invariant" in stdout:
            info["initial_state_check"] = True
        else:
            info["initial_state_check"] = False
    
    # For non-serializable cases, check for counterexample
    elif info["result"] == "not_serializable":
        if "NS-Level Counterexample Trace:" in stdout:
            info["has_ns_trace"] = True
        else:
            info["has_ns_trace"] = False
            
        if "Completed Request/Response Pairs:" in stdout:
            info["has_completed_pairs"] = True
        else:
            info["has_completed_pairs"] = False
    
    info["status"] = "success"
    return info

def main():
    json_dir = Path("examples/json/small")
    json_files = sorted(json_dir.glob("*.json"))
    
    results = {}
    problematic_files = []
    
    print(f"Testing {len(json_files)} JSON files...")
    print("=" * 80)
    
    for i, json_file in enumerate(json_files):
        print(f"\n[{i+1}/{len(json_files)}] Testing {json_file.name}...", flush=True)
        
        stdout, stderr, returncode = run_serializability_checker(str(json_file))
        info = analyze_result(stdout, stderr, returncode)
        
        results[json_file.name] = info
        
        # Identify problematic cases
        is_problematic = False
        problem_desc = []
        
        if info["status"] == "error":
            is_problematic = True
            problem_desc.append(f"Error: {info.get('error', 'Unknown error')}")
        elif info["result"] == "serializable":
            if info.get("proof_valid") is False:
                is_problematic = True
                problem_desc.append("Invalid proof certificate")
            elif info.get("proof_valid") is None:
                is_problematic = True
                problem_desc.append("No proof verification performed")
            elif not info.get("has_ns_invariants"):
                is_problematic = True
                problem_desc.append("Missing NS-level invariants")
            elif not info.get("initial_state_check"):
                is_problematic = True
                problem_desc.append("Initial state check failed or missing")
        elif info["result"] == "not_serializable":
            if not info.get("has_ns_trace"):
                is_problematic = True
                problem_desc.append("Missing NS-level counterexample trace")
            if not info.get("has_completed_pairs"):
                is_problematic = True
                problem_desc.append("Missing completed request/response pairs")
        elif info["result"] == "unknown":
            is_problematic = True
            problem_desc.append("Could not determine serializability result")
        
        if is_problematic:
            problematic_files.append({
                "file": json_file.name,
                "info": info,
                "problems": problem_desc
            })
            print(f"  ❌ PROBLEMATIC: {', '.join(problem_desc)}")
        else:
            print(f"  ✅ OK: {info['result']}")
    
    # Summary
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    
    total = len(json_files)
    errors = sum(1 for r in results.values() if r["status"] == "error")
    serializable = sum(1 for r in results.values() if r.get("result") == "serializable")
    not_serializable = sum(1 for r in results.values() if r.get("result") == "not_serializable")
    unknown = sum(1 for r in results.values() if r.get("result") == "unknown")
    
    print(f"Total files tested: {total}")
    print(f"Errors: {errors}")
    print(f"Serializable: {serializable}")
    print(f"Not serializable: {not_serializable}")
    print(f"Unknown result: {unknown}")
    print(f"Problematic files: {len(problematic_files)}")
    
    if problematic_files:
        print("\n" + "=" * 80)
        print("PROBLEMATIC FILES DETAILS")
        print("=" * 80)
        
        for pf in problematic_files:
            print(f"\n{pf['file']}:")
            print(f"  Result: {pf['info'].get('result', 'N/A')}")
            print(f"  Status: {pf['info'].get('status', 'N/A')}")
            print(f"  Problems:")
            for problem in pf['problems']:
                print(f"    - {problem}")
            if pf['info'].get('status') == 'error':
                print(f"  Error details: {pf['info'].get('error', 'N/A')}")
    
    # Save detailed results
    with open("proof_check_results.json", "w") as f:
        json.dump(results, f, indent=2)
    
    print(f"\nDetailed results saved to proof_check_results.json")
    
    return len(problematic_files)

if __name__ == "__main__":
    sys.exit(main())