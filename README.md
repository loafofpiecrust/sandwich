# Sandwich
Machines in a room having a conversation about sandwiches.

## Operation
All machines run the same program, which simultaneously scans for peers to talk to about sandwiches and keeps a server open listening for peers trying to connect.
All machines have the same list of hostnames that represents the full roster of machines, letting them request conversations specifically with their peers.

# Resources

## Creative Resources
- I'm using tones based off of dial tones to sound each syllable. [DTMF Tones](http://www.dialabc.com/sound/dtmf.html)
- [This artist's work](https://www.istockphoto.com/portfolio/bad_arithmetic?assettype=image&sort=mostpopular) is similar to how I've been imagining the sandwich imagery.

## Linguistic Resources
- [Atlas of Language Structures](https://wals.info/)

## Technical Resources
- [Cross-compiling Rust for Raspberry Pi](https://hackernoon.com/compiling-rust-for-the-raspberry-pi-49fdcd7df658)
- [SSH on RPI](https://www.raspberrypi.org/documentation/remote-access/ssh/)
- We want to access all our devices on a local network without doing any scanning.
  Simply [run an mDNS service](https://www.howtogeek.com/167190/how-and-why-to-assign-the-.local-domain-to-your-raspberry-pi/) to access that device by `hostname.local`.
- [This crate](https://docs.rs/programinduction/0.6.9/programinduction/index.html) for "Bayesian program learning" may be useful for language representation as PCFG.

## Development
This project is written in Rust managed by Cargo, so to build everything simply run:
```sh
cargo build
```

All the machines connect by hostname on the port `34222` to keep everything simple.


## Sandwiches
### Types of Sandwiches
- American: sliced bread
  - Bagel sandwich, breakfast sandwich
- Italian: small rolls sliced in half
  - subs
- Quesadilla: different size and cheese melted so hard to compose.
- Gyro
- bahn mi

### The Menu
- Cheeses: cheddar, fresh mozzerella, swiss, provolone, pecorino, feta
- Proteins: egg, bacon, turkey, falafel, mushroom, ham, pulled pork, salmon, salami, roast beef, beef patty
- Greens: spinach, arugula, lettuce, cilantro
- Vegis: green bell pepper, red bell pepper, jalapeno
- Fruit: tomato, avocado, apple slices, cucumber, eggplant, olives
- Sauce: mayo, mustard, ketchup, barbeque, gravy, tzatziki
- Spreads: cream cheese, hummus, pesto
- Other: red onion, pickled onion, pickled radish, shredded carrot, onion ring
- Bread: everything bagel, focaccia, baguette, sourdough slices, wheat, english muffin, tortilla, pita

### Starter Sandwiches
- Breakfast Bagel: everything bagel, fried egg, swiss cheese, avocado
