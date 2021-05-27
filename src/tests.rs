use crate::feature::Feature;
use anyhow::Result;
#[test]
fn test_load_feature() -> Result<()> {
    let input = r###"
    Feature: Farm activities
    
    
    Scenario: Shave a yak
        Given I have a yak
        And My yak has hair
        And I have a razor
        When I shave the yak
        Then My yak does not have <hair>
        And I have yak hair
    
    
    
    Scenario Outline: Shave an animal
        Given I am Old McDonald
        And I have a farm
        And On that farm there is a <animal>
        When I listen
        Then I hear a <noise> here
        And I hear a <noise> there
    Examples:
        | animal | noise |
        | cow    | moo   |
        | horse  | neigh |
        | pig    | oink  |
    "###;
    Feature::from_str(input).map(|_| ())
}

#[test]
fn test_load_outline_with_multiple_example_blocks() -> Result<()> {
    let input = r###"
    Feature: Farm activities
    
    
    Scenario: Shave a yak
        Given I have a yak
        And My yak has hair
        And I have a razor
        When I shave the yak
        Then My yak does not have <hair>
        And I have yak hair
    
    
    
    Scenario Outline: Shave an animal
        Given I am Old McDonald
        And I have a farm
        And On that farm there is a <animal>
        When I listen
        Then I hear a <noise> here
        And I hear a <noise> there
    
    @Mammal
    Examples:
        | animal  | noise |
        | cow     | moo   |
        | horse   | neigh |
        | pig     | oink  |
    
    @Bird
    Examples:
        | duck    | quack |
        | chicken | cluck |
    "###;
    Feature::from_str(input).map(|_| ())
}
