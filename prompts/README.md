# KOTA Prompt System

This directory contains structured prompts that define KOTA's behavior, capabilities, and interaction patterns. These prompts are always loaded when KOTA starts, providing consistent behavior and guidance across all operations.

## Overview

The KOTA prompt system is inspired by tools like Aider but adapted for KOTA's unique philosophy of self-improvement and cognitive partnership. Each prompt file serves a specific purpose and can be modified to customize KOTA's behavior.

## Prompt Files

### base_prompts.toml
Core system prompts that define KOTA's fundamental behavior and personality.

**Sections:**
- `[identity]` - Defines KOTA's core characteristics and principles
- `[capabilities]` - Guidelines for file operations and command execution
- `[interaction_style]` - Response formatting and error handling
- `[self_modification]` - Safety checks and guidelines for self-improvement
- `[learning]` - Context awareness and adaptation strategies

### editing_prompts.toml
Different code editing modes and strategies for modifying files.

**Sections:**
- `[search_replace]` - Precise editing using SEARCH/REPLACE blocks
- `[whole_file]` - Complete file replacement strategy
- `[command_generation]` - Shell command creation guidelines
- `[diff_patch]` - Unified diff format for changes
- `[hybrid_mode]` - Intelligent mode selection
- `[validation]` - Pre and post-edit checks
- `[best_practices]` - General editing guidelines

### self_modification_prompts.toml
Guidelines specific to KOTA's self-improvement capabilities.

**Sections:**
- `[core_principles]` - Philosophy and ethics of self-modification
- `[analysis_phase]` - Self-assessment and impact analysis
- `[implementation_strategies]` - Approaches to enhancement
- `[modification_workflow]` - Step-by-step modification process
- `[common_enhancements]` - Typical improvement areas
- `[testing_approach]` - Verification and rollback strategies
- `[evolution_patterns]` - Long-term growth strategies

### code_understanding_prompts.toml
Prompts for analyzing, explaining, and understanding code.

**Sections:**
- `[analysis_modes]` - Overview vs deep-dive approaches
- `[explanation_styles]` - Adapting to audience expertise
- `[code_review]` - Systematic review processes
- `[pattern_recognition]` - Identifying patterns and anti-patterns
- `[dependency_analysis]` - Managing project dependencies
- `[documentation_generation]` - Creating useful documentation
- `[debugging_assistance]` - Helping solve problems
- `[performance_analysis]` - Optimization strategies
- `[security_review]` - Identifying vulnerabilities

### planning_prompts.toml
Task organization and project planning guidance.

**Sections:**
- `[task_decomposition]` - Breaking down complex requests
- `[planning_strategies]` - Different approaches to task execution
- `[project_organization]` - Structuring codebases effectively
- `[estimation_techniques]` - Complexity and time assessment
- `[risk_management]` - Identifying and mitigating risks
- `[progress_tracking]` - Monitoring task completion
- `[collaboration_planning]` - Working effectively with others
- `[execution_patterns]` - Common implementation approaches
- `[decision_making]` - Making and documenting choices
- `[continuous_improvement]` - Learning from experience

## Usage

These prompts are automatically loaded when KOTA starts. They provide context and guidance for KOTA's behavior but don't override direct user instructions.

### Customization

You can customize KOTA's behavior by editing these files:

1. Modify specific sections to change behavior in those areas
2. Add new sections for additional capabilities
3. Create new prompt files for specialized domains

### Best Practices

1. **Keep prompts focused** - Each section should have a clear purpose
2. **Use clear language** - Prompts should be unambiguous
3. **Maintain consistency** - Similar tasks should have similar guidance
4. **Document changes** - Note why prompts were modified
5. **Test modifications** - Ensure changes produce desired behavior

## Integration with KOTA

The prompt system integrates with KOTA's architecture:

1. **Context System** - Prompts can reference files in context
2. **Command Parser** - Prompts guide command interpretation
3. **LLM Integration** - Prompts shape model interactions
4. **Self-Modification** - Prompts can be updated by KOTA itself

## Future Enhancements

Potential improvements to the prompt system:

1. **Dynamic Loading** - Load prompts based on task type
2. **User Profiles** - Custom prompts per user/project
3. **Prompt Versioning** - Track prompt evolution
4. **Performance Metrics** - Measure prompt effectiveness
5. **Domain Specialization** - Industry-specific prompts

## Contributing

When adding new prompts:

1. Follow the existing TOML structure
2. Use descriptive section names
3. Include examples where helpful
4. Document the prompt's purpose
5. Test with various scenarios

## Prompt Philosophy

The KOTA prompt system embodies several key principles:

1. **Empowerment** - Enable users to accomplish more
2. **Transparency** - Clear about capabilities and limitations
3. **Adaptability** - Flexible to different working styles
4. **Safety** - Protective of user data and system integrity
5. **Growth** - Designed to evolve and improve

These prompts form the foundation of KOTA's behavior, making it a reliable and powerful cognitive partner for software development.