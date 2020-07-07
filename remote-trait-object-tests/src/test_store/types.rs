// Copyright 2020 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use remote_trait_object::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Pizza {
    Pepperoni,
    Veggie,
    Pineapple,
}

#[rto_macro::service]
pub trait CreditCard: Service {
    fn pay(&mut self, money: u32) -> Result<(), ()>;
}

#[rto_macro::service]
pub trait Store: Service {
    fn order_pizza(&self, menu: Pizza, money: u32) -> String;
    fn order_coke(&self, flavor: &str, money: u32) -> String;
    fn order_pizza_credit_card(&self, menu: Pizza, credit_card: ServiceRef<dyn CreditCard>) -> String;
}
